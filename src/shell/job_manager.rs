use std::fmt;

use log::error;
use nix::sys::wait::waitpid;
use nix::sys::wait::WaitPidFlag as WF;
use nix::sys::wait::WaitStatus as WS;
use nix::unistd::Pid;

use super::signals;

#[derive(Debug, Clone)]
pub enum JobStatus {
    Done,
    Killed,
    Continued,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub gid: i32,
    pub pid: i32,
    pub index: usize,
    pub command: String,
    pub status: JobStatus,
    pub is_bg: bool,
    pub is_current: bool,
    pub is_previous: bool,
}

impl Job {
    fn new(gid: i32, pid: i32, index: usize, command: String, status: JobStatus) -> Self {
        Self {
            gid,
            pid,
            index,
            command,
            status,
            is_bg: false,
            is_current: false,
            is_previous: false,
        }
    }
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = match self.status {
            JobStatus::Done => "done",
            JobStatus::Killed => "killed",
            JobStatus::Continued => "continued",
            JobStatus::Stopped => "stopped",
        };
        let mark = if self.is_current {
            "+"
        } else if self.is_previous {
            "-"
        } else {
            " "
        };
        write!(
            f,
            "[{}] {} {} {} {}",
            self.index, mark, self.pid, status, self.command
        )
    }
}

#[derive(Clone)]
pub struct JobManager {
    jobs: Vec<Job>,
}

impl JobManager {
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    pub fn get_jobs(&self) -> &Vec<Job> {
        &self.jobs
    }

    fn find_available_index(&self) -> usize {
        let mut index = 1;
        while self.jobs.iter().any(|job| job.index == index) {
            index += 1;
        }
        index
    }

    pub fn add_job(&mut self, gid: i32, pid: i32, command: String) {
        let index = self.find_available_index();
        // 将当前任务变为上一个任务
        for job in self.jobs.iter_mut() {
            if job.is_current {
                job.is_current = false;
                job.is_previous = true;
            } else {
                job.is_previous = false;
            }
        }

        let mut job = Job::new(gid, pid, index, command, JobStatus::Continued);
        job.is_current = true;
        self.jobs.push(job);
        self.update_marks(index);
    }

    pub fn remove_job(&mut self, _gid: i32, pid: i32) -> Option<Job> {
        if let Some(pos) = self.jobs.iter().position(|job| job.pid == pid) {
            let was_current = self.jobs[pos].is_current;
            let current_job = self.jobs[pos].clone();
            self.jobs.remove(pos);

            if was_current && !self.jobs.is_empty() {
                // 如果删除的是当前任务，将上一个任务提升为当前任务
                if let Some(prev_job) = self.jobs.iter_mut().find(|job| job.is_previous) {
                    prev_job.is_current = true;
                    prev_job.is_previous = false;
                } else {
                    // 如果没有上一个任务，将最后一个任务设为当前任务
                    let last_idx = self.jobs.len() - 1;
                    self.jobs[last_idx].is_current = true;
                }
            }
            return Some(current_job);
        }
        None
    }

    pub fn fg(&mut self, index: Option<usize>) -> Option<Job> {
        let pos = match index {
            Some(idx) => self.jobs.iter().position(|job| job.index == idx)?,
            None => self.jobs.iter().position(|job| job.is_current)?,
        };

        let job_index = self.jobs[pos].index;
        self.jobs[pos].status = JobStatus::Continued;

        self.update_marks(job_index);
        Some(self.jobs[pos].clone())
    }

    pub fn bg(&mut self, index: Option<usize>) -> Option<Job> {
        let pos = match index {
            Some(idx) => self.jobs.iter().position(|job| job.index == idx)?,
            None => self.jobs.iter().position(|job| job.is_current)?,
        };
        self.jobs[pos].status = JobStatus::Continued;
        Some(self.jobs[pos].clone())
    }

    fn update_marks(&mut self, current_job_index: usize) {
        for job in self.jobs.iter_mut() {
            if job.index == current_job_index {
                job.is_current = true;
                job.is_previous = false;
            } else if job.is_current {
                job.is_current = false;
                job.is_previous = true;
            } else {
                job.is_previous = false;
            }
        }
    }

    fn mark_job_as_done(&mut self, gid: i32, pid: i32, status: JobStatus) {
        if let Some(mut job) = self.remove_job(gid, pid) {
            job.status = status;
            if job.is_bg {
                println!("");
                println!("{}", &job);
            }
        }
    }

    fn mark_job_stopped(&mut self, _gid: i32, pid: i32, report: bool) {
        let job = self.jobs.iter_mut().find(|job| job.pid == pid);
        match job {
            Some(job) => {
                job.status = JobStatus::Stopped;
                job.is_bg = true;
                if report {
                    println!("");
                    println!("{}", &job);
                }
            }
            None => {}
        }
    }

    pub fn waitpidx(&self, wpid: i32, block: bool) -> WaitStatus {
        let options = if block {
            Some(WF::WUNTRACED | WF::WCONTINUED)
        } else {
            Some(WF::WUNTRACED | WF::WCONTINUED | WF::WNOHANG)
        };
        match waitpid(Pid::from_raw(wpid), options) {
            Ok(WS::Exited(pid, status)) => {
                let pid = i32::from(pid);
                WaitStatus::from_exited(pid, status)
            }
            Ok(WS::Stopped(pid, sig)) => {
                let pid = i32::from(pid);
                WaitStatus::from_stopped(pid, sig as i32)
            }
            Ok(WS::Continued(pid)) => {
                let pid = i32::from(pid);
                WaitStatus::from_continuted(pid)
            }
            Ok(WS::Signaled(pid, sig, _core_dumped)) => {
                let pid = i32::from(pid);
                WaitStatus::from_signaled(pid, sig as i32)
            }
            Ok(WS::StillAlive) => WaitStatus::empty(),
            Err(e) => WaitStatus::from_error(e as i32),
        }
    }

    pub fn wait_fg_job(&mut self, gid: i32, pids: &[i32]) -> CommandResult {
        let mut cmd_result = CommandResult::new();
        let mut count_waited = 0;
        let count_child = pids.len();
        if count_child == 0 {
            return cmd_result;
        }
        let pid_last = pids.last().unwrap();

        loop {
            let ws = self.waitpidx(-1, true);
            // here when we calling waitpidx(), all signals should have
            // been masked. There should no errors (ECHILD/EINTR etc) happen.
            if ws.is_error() {
                let err = ws.get_errno();
                if err == nix::Error::ECHILD {
                    break;
                }

                error!("jobc unexpected waitpid error: {}", err);
                cmd_result = CommandResult::from_status(gid, err as i32);
                break;
            }

            let pid = ws.get_pid();
            let is_a_fg_child = pids.contains(&pid);
            if is_a_fg_child && !ws.is_continued() {
                count_waited += 1;
            }

            if ws.is_exited() {
                if is_a_fg_child {
                    self.mark_job_as_done(gid, pid, JobStatus::Done);
                } else {
                    let status = ws.get_status();
                    signals::insert_reap_map(pid, status);
                }
            } else if ws.is_stopped() {
                if is_a_fg_child {
                    // for stop signal of fg job (current job)
                    // i.e. Ctrl-Z is pressed on the fg job
                    self.mark_job_stopped(gid, pid, true);
                } else {
                    // for stop signal of bg jobs
                    signals::insert_stopped_map(pid);
                    self.mark_job_stopped(0, pid, false);
                }
            } else if ws.is_continued() {
                if !is_a_fg_child {
                    signals::insert_cont_map(pid);
                }
                continue;
            } else if ws.is_signaled() {
                if is_a_fg_child {
                    self.mark_job_as_done(gid, pid, JobStatus::Killed);
                } else {
                    signals::killed_map_insert(pid, ws.get_signal());
                }
            }

            if is_a_fg_child && pid == *pid_last {
                let status = ws.get_status();
                cmd_result.status = status;
            }

            if count_waited >= count_child {
                break;
            }
        }
        cmd_result
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct CommandResult {
    pub gid: i32,
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    pub fn new() -> CommandResult {
        CommandResult {
            gid: 0,
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    pub fn from_status(gid: i32, status: i32) -> CommandResult {
        CommandResult {
            gid,
            status,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    pub fn error() -> CommandResult {
        CommandResult {
            gid: 0,
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WaitStatus(i32, i32, i32);

impl WaitStatus {
    pub fn from_exited(pid: i32, status: i32) -> Self {
        WaitStatus(pid, 0, status)
    }

    pub fn from_signaled(pid: i32, sig: i32) -> Self {
        WaitStatus(pid, 1, sig)
    }

    pub fn from_stopped(pid: i32, sig: i32) -> Self {
        WaitStatus(pid, 2, sig)
    }

    pub fn from_continuted(pid: i32) -> Self {
        WaitStatus(pid, 3, 0)
    }

    pub fn from_others() -> Self {
        WaitStatus(0, 9, 9)
    }

    pub fn from_error(errno: i32) -> Self {
        WaitStatus(0, 255, errno)
    }

    pub fn empty() -> Self {
        WaitStatus(0, 0, 0)
    }

    pub fn is_error(&self) -> bool {
        self.1 == 255
    }

    pub fn is_others(&self) -> bool {
        self.1 == 9
    }

    pub fn is_signaled(&self) -> bool {
        self.1 == 1
    }

    pub fn get_errno(&self) -> nix::Error {
        nix::Error::from_raw(self.2)
    }

    pub fn is_exited(&self) -> bool {
        self.0 != 0 && self.1 == 0
    }

    pub fn is_stopped(&self) -> bool {
        self.1 == 2
    }

    pub fn is_continued(&self) -> bool {
        self.1 == 3
    }

    pub fn get_pid(&self) -> i32 {
        self.0
    }

    fn _get_signaled_status(&self) -> i32 {
        self.2 + 128
    }

    pub fn get_signal(&self) -> i32 {
        self.2
    }

    pub fn get_name(&self) -> String {
        if self.is_exited() {
            "Exited".to_string()
        } else if self.is_stopped() {
            "Stopped".to_string()
        } else if self.is_continued() {
            "Continued".to_string()
        } else if self.is_signaled() {
            "Signaled".to_string()
        } else if self.is_others() {
            "Others".to_string()
        } else if self.is_error() {
            "Error".to_string()
        } else {
            format!("unknown: {}", self.2)
        }
    }

    pub fn get_status(&self) -> i32 {
        if self.is_exited() {
            self.2
        } else {
            self._get_signaled_status()
        }
    }
}

impl fmt::Debug for WaitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut formatter = f.debug_struct("WaitStatus");
        formatter.field("pid", &self.0);
        let name = self.get_name();
        formatter.field("name", &name);
        formatter.field("ext", &self.2);
        formatter.finish()
    }
}
