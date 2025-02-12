use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone)]
pub struct Scheduler {
    inner: Arc<(Mutex<bool>, Condvar)>
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            inner: Arc::new((Mutex::new(true), Condvar::new()))
        }
    }

    /// 等待直到成为前台进程组
    pub fn wait_until_foreground(&self) {
        let (lock, cvar) = &*self.inner;
        let mut is_fg = lock.lock().unwrap();
        while !*is_fg {
            is_fg = cvar.wait(is_fg).unwrap();
        }
    }

    /// 暂停当前 shell（让出前台进程组）
    pub fn pause(&self) {
        let (lock, cvar) = &*self.inner;
        let mut is_fg = lock.lock().unwrap();
        *is_fg = false;
        cvar.notify_all();
    }

    /// 恢复当前 shell 为前台进程组
    pub fn resume(&self) {
        let (lock, cvar) = &*self.inner;
        let mut is_fg = lock.lock().unwrap();
        *is_fg = true;
        cvar.notify_all();
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
} 