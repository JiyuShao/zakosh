use std::fmt;

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

pub struct JobManager {
    jobs: Vec<Job>,
}

impl JobManager {
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    fn find_available_index(&self) -> usize {
        let mut index = 1;
        while self.jobs.iter().any(|job| job.index == index) {
            index += 1;
        }
        index
    }

    pub fn add_job(&mut self, pid: u32, command: String) {
        // 将当前任务变为上一个任务
        for job in &mut self.jobs {
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
        self.jobs.push(job);
        self.update_marks(index);
    }

    pub fn remove_job(&mut self, pid: u32) {
        if let Some(pos) = self.jobs.iter().position(|job| job.pid == pid) {
            let was_current = self.jobs[pos].is_current;
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
        }
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

    pub fn bg(&mut self, index: usize) -> Option<Job> {
        let pos = self.jobs.iter().position(|job| job.index == index)?;
        let job_index = self.jobs[pos].index;
        self.jobs[pos].status = JobStatus::Continued;

        self.update_marks(job_index);
        Some(self.jobs[pos].clone())
    }

    pub fn mark_suspended(&mut self, pid: u32) -> Option<Job> {
        let job = self.jobs.iter_mut().find(|job| job.pid == pid)?;
        job.status = JobStatus::Suspended;
        Some(job.clone())
    }

    pub fn get_jobs(&self) -> Vec<Job> {
        self.jobs.clone()
    }

    fn update_marks(&mut self, current_job_index: usize) {
        for job in &mut self.jobs {
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
