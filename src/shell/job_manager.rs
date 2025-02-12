use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum JobStatus {
    Continued,
    Suspended,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub pid: u32,
    pub index: usize,
    pub command: String,
    pub status: JobStatus,
    pub is_current: bool,
    pub is_previous: bool,
}

impl Job {
    fn new(pid: u32, index: usize, command: String, status: JobStatus) -> Self {
        Self {
            pid,
            index,
            command,
            status,
            is_current: false,
            is_previous: false,
        }
    }
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = match self.status {
            JobStatus::Continued => "continued",
            JobStatus::Suspended => "suspended",
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
    inner: Arc<Mutex<Vec<Job>>>,
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn find_available_index(&self) -> usize {
        let mut index = 1;
        let jobs = self.inner.lock().unwrap();
        while jobs.iter().any(|job| job.index == index) {
            index += 1;
        }
        index
    }

    pub fn add_job(&self, pid: u32, command: String) {
        let mut jobs = self.inner.lock().unwrap();
        // 将当前任务变为上一个任务
        for job in jobs.iter_mut() {
            if job.is_current {
                job.is_current = false;
                job.is_previous = true;
            } else {
                job.is_previous = false;
            }
        }

        let index = self.find_available_index();
        let mut job = Job::new(pid, index, command, JobStatus::Continued);
        job.is_current = true;
        jobs.push(job);
        self.update_marks(index);
    }

    pub fn remove_job(&self, pid: u32) {
        let mut jobs = self.inner.lock().unwrap();
        if let Some(pos) = jobs.iter().position(|job| job.pid == pid) {
            let was_current = jobs[pos].is_current;
            jobs.remove(pos);

            if was_current && !jobs.is_empty() {
                // 如果删除的是当前任务，将上一个任务提升为当前任务
                if let Some(prev_job) = jobs.iter_mut().find(|job| job.is_previous) {
                    prev_job.is_current = true;
                    prev_job.is_previous = false;
                } else {
                    // 如果没有上一个任务，将最后一个任务设为当前任务
                    let last_idx = jobs.len() - 1;
                    jobs[last_idx].is_current = true;
                }
            }
        }
    }

    pub fn fg(&self, index: Option<usize>) -> Option<Job> {
        let jobs = self.inner.lock().unwrap();
        let pos = match index {
            Some(idx) => jobs.iter().position(|job| job.index == idx)?,
            None => jobs.iter().position(|job| job.is_current)?,
        };

        let job_index = jobs[pos].index;
        let mut jobs = self.inner.lock().unwrap();
        jobs[pos].status = JobStatus::Continued;

        self.update_marks(job_index);
        Some(jobs[pos].clone())
    }

    pub fn bg(&self, index: Option<usize>) -> Option<Job> {
        let jobs = self.inner.lock().unwrap();
        let pos = match index {
            Some(idx) => jobs.iter().position(|job| job.index == idx)?,
            None => jobs.iter().position(|job| job.is_current)?,
        };
        let job_index = jobs[pos].index;
        let mut jobs = self.inner.lock().unwrap();
        jobs[pos].status = JobStatus::Continued;

        self.update_marks(job_index);
        Some(jobs[pos].clone())
    }

    pub fn mark_suspended(&self, pid: u32) -> Option<Job> {
        let mut jobs = self.inner.lock().unwrap();
        let job = jobs.iter_mut().find(|job| job.pid == pid)?;
        job.status = JobStatus::Suspended;
        Some(job.clone())
    }

    pub fn get_jobs(&self) -> Vec<Job> {
        self.inner.lock().unwrap().clone()
    }

    fn update_marks(&self, current_job_index: usize) {
        let mut jobs = self.inner.lock().unwrap();
        for job in jobs.iter_mut() {
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
}
