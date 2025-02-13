use log::{debug, trace};
use once_cell::sync::OnceCell;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::Mutex;
use std::{env, io};

use crate::shell::job_manager::JobManager;
use crate::shell::parser::ast::{Command as ShellCommand, Node};
use crate::shell::scheduler::Scheduler;

use super::variable::Variable;

static JOB_MANAGER: OnceCell<Mutex<Arc<JobManager>>> = OnceCell::new();
static SCHEDULER: OnceCell<Arc<Scheduler>> = OnceCell::new();

pub struct Executor {
    variables: Variable,
    job_manager: Arc<JobManager>,
    scheduler: Arc<Scheduler>,
}

impl Executor {
    pub fn new(scheduler: Arc<Scheduler>, job_manager: Arc<JobManager>) -> Self {
        let _ = SCHEDULER.set(Arc::clone(&scheduler)); // 存储 scheduler 的克隆
        let _ = JOB_MANAGER.set(Mutex::new(Arc::clone(&job_manager)));

        #[cfg(unix)]
        unsafe {
            // shell 进程忽略这些信号
            libc::signal(libc::SIGINT, libc::SIG_IGN); // Ctrl-C
            libc::signal(libc::SIGQUIT, libc::SIG_IGN); // Ctrl-\
            libc::signal(libc::SIGTSTP, libc::SIG_IGN); // Ctrl-Z
            libc::signal(libc::SIGTTOU, libc::SIG_IGN); // 当后台进程尝试写入终端时不暂停进程
            libc::signal(libc::SIGTTIN, libc::SIG_IGN); // 当后台进程尝试从终端读取输入时不暂停进程

            // 监听子进程的变化
            // 设置 SIGCHLD 处理函数
            extern "C" fn handle_sigchld(_: libc::c_int) {
                unsafe {
                    loop {
                        let mut status: libc::c_int = 0;
                        match libc::waitpid(
                            -1,
                            &mut status,
                            libc::WNOHANG | libc::WUNTRACED | libc::WCONTINUED,
                        ) {
                            0 | -1 => break,
                            pid => {
                                if let Some(job_manager) = JOB_MANAGER.get() {
                                    if let Ok(manager) = job_manager.lock() {
                                        if libc::WIFCONTINUED(status) {
                                            // 进程继续运行
                                            println!("\nJob {} continued", pid);
                                            trace!("Job {} continued", pid);
                                            break;
                                        }
                                        if libc::WIFSTOPPED(status) {
                                            trace!("Job {} suspended", pid);
                                            if let Some(job) = manager.mark_suspended(pid as u32) {
                                                println!("\nJob {} suspended: {}", pid, job);
                                            }
                                        } else if libc::WIFEXITED(status) {
                                            let exit_code = libc::WEXITSTATUS(status);
                                            println!(
                                                "\nJob {} completed with exit code {}",
                                                pid, exit_code
                                            );
                                            trace!(
                                                "Job {} completed with exit code {}",
                                                pid,
                                                exit_code
                                            );
                                            manager.remove_job(pid as u32);
                                        } else if libc::WIFSIGNALED(status) {
                                            let signal = libc::WTERMSIG(status);
                                            println!(
                                                "\nJob {} terminated by signal {}",
                                                pid, signal
                                            );
                                            trace!("Job {} terminated by signal {}", pid, signal);
                                            manager.remove_job(pid as u32);
                                            // 如果是被信号终止，打印一个换行，因为信号处理可能打断了输出
                                            println!();
                                        } else {
                                            trace!(
                                                "Job {} stopped with unknown raw status {}",
                                                pid,
                                                status
                                            );
                                        }

                                        // 恢复 shell 为前台进程组
                                        let shell_terminal = libc::STDIN_FILENO;
                                        let shell_pgid = libc::getpid();
                                        libc::tcsetpgrp(shell_terminal, shell_pgid);
                                        trace!("恢复 {} 为前台进程组", shell_pgid);

                                        // 通知 scheduler shell 已经恢复为前台进程组
                                        if let Some(scheduler) = SCHEDULER.get() {
                                            scheduler.resume();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            libc::signal(libc::SIGCHLD, handle_sigchld as libc::sighandler_t);
        }

        Self {
            variables: Variable::new(),
            job_manager: job_manager,
            scheduler: scheduler,
        }
    }

    pub fn execute(&mut self, node: Node) -> io::Result<()> {
        match node {
            Node::Pipeline(pipeline) => self.execute_pipeline(pipeline),
            Node::Command(command) => self.execute_command(command),
        }
    }

    fn execute_pipeline(&mut self, pipeline: Vec<ShellCommand>) -> io::Result<()> {
        // 暂时只处理单个命令，后续可以扩展管道功能
        if let Some(command) = pipeline.first() {
            self.execute_command(command.clone())
        } else {
            Ok(())
        }
    }

    fn execute_command(&mut self, command: ShellCommand) -> io::Result<()> {
        // 处理内建命令
        if let Some(result) = self.handle_builtin(&command) {
            debug!("执行内建命令: {:?}", command);
            return result;
        }

        // 执行外部命令
        debug!("执行外部命令: {:?}", command);
        let program = command.program;
        let original_args = command.arguments.clone();
        let args: Vec<String> = command
            .arguments
            .iter()
            .map(|arg| self.expand_variables(arg))
            .collect();

        let mut command = Command::new(&program);

        #[cfg(unix)]
        unsafe {
            command.pre_exec(|| {
                // 子进程恢复默认的信号处理
                libc::signal(libc::SIGINT, libc::SIG_DFL); // Ctrl-C
                libc::signal(libc::SIGQUIT, libc::SIG_DFL); // Ctrl-\
                libc::signal(libc::SIGTSTP, libc::SIG_DFL); // Ctrl-Z
                libc::signal(libc::SIGTTOU, libc::SIG_DFL); // 当后台进程尝试写入终端时暂停进程
                libc::signal(libc::SIGTTIN, libc::SIG_DFL); // 当后台进程尝试从终端读取输入时暂停进程

                // 在子进程启动时立即设置进程组
                let pid = libc::getpid();
                if libc::setpgid(pid, pid) < 0 {
                    // 使用 0,0 让子进程成为自己的进程组长
                    return Err(io::Error::last_os_error());
                }

                Ok(())
            });
        }

        let child = command
            .args(args.clone())
            .env_clear()
            .envs(self.variables.get_all())
            .current_dir(env::current_dir().unwrap_or_default())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        let pid = child.id();
        trace!("运行外部命令[{}]: {} {:?}", pid, program, args);

        #[cfg(unix)]
        unsafe {
            // 在父进程中也尝试设置子进程的进程组
            // 这是为了处理竞态条件
            let _ = libc::setpgid(pid as libc::pid_t, pid as libc::pid_t);

            // 将子进程设为前台进程组
            let shell_terminal = libc::STDIN_FILENO;
            if libc::tcsetpgrp(shell_terminal, pid as libc::pid_t) < 0 {
                trace!("设置前台进程组失败: {}", io::Error::last_os_error());
            }
            trace!("将子进程 {} 设为前台进程组", pid);
        }
        trace!("Job {} started", pid);

        self.job_manager.add_job(
            pid,
            program.clone()
                + " "
                + original_args
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
                    .as_str(),
        );

        // 暂停 shell，等待子进程运行完成或被放到后台
        self.scheduler.pause();

        Ok(())
    }

    fn expand_variables(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' && chars.peek().is_some() {
                let mut var_name = String::new();
                while let Some(&next_char) = chars.peek() {
                    if next_char.is_alphanumeric() || next_char == '_' {
                        var_name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if !var_name.is_empty() {
                    result.push_str(&self.variables.get(var_name));
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    // 处理内建命令
    fn handle_builtin(&mut self, command: &ShellCommand) -> Option<io::Result<()>> {
        match command.program.as_str() {
            "zako" => Some(self.builtin_shell()),
            "cd" => Some(self.builtin_cd(command)),
            "exit" => Some(self.builtin_exit()),
            "set" => Some(self.builtin_set(command)),
            "jobs" => Some(self.builtin_jobs()),
            "fg" => Some(self.builtin_fg(command)),
            "bg" => Some(self.builtin_bg(command)),
            _ => None,
        }
    }

    fn builtin_shell(&self) -> io::Result<()> {
        let executable = env::current_exe().unwrap_or_default();
        let _ = Command::new(executable)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
            .wait();
        Ok(())
    }

    fn builtin_cd(&mut self, command: &ShellCommand) -> io::Result<()> {
        let path = command.arguments.first().map(|s| s.as_str()).unwrap_or("~");
        let path = shellexpand::tilde(path);
        std::env::set_current_dir(path.as_ref())
    }

    fn builtin_exit(&self) -> io::Result<()> {
        std::process::exit(0);
    }

    fn builtin_set(&mut self, command: &ShellCommand) -> io::Result<()> {
        if command.arguments.len() != 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "set 命令需要两个参数: 变量名和值",
            ));
        }

        self.variables
            .set(command.arguments[0].clone(), command.arguments[1].clone());
        Ok(())
    }

    fn builtin_jobs(&self) -> io::Result<()> {
        for job in &self.job_manager.get_jobs() {
            println!("{}", job);
        }
        Ok(())
    }

    fn builtin_fg(&mut self, command: &ShellCommand) -> io::Result<()> {
        let index =
            if let Some(arg) = command.arguments.first() {
                // 处理 %n 格式
                if let Some(index_str) = arg.strip_prefix('%') {
                    Some(index_str.parse::<usize>().map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidInput, "无效的作业编号")
                    })?)
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "fg: 参数格式错误，应为 %n",
                    ));
                }
            } else {
                None
            };

        let job = self
            .job_manager
            .fg(index)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "fg: 没有找到该作业"))?;
        println!("{}", job);

        let pid = job.pid;

        // 暂停当前 shell，等待子进程运行完成或被放到后台
        self.scheduler.pause();

        #[cfg(unix)]
        unsafe {
            // 将作业设置为前台进程组
            libc::tcsetpgrp(0, pid as libc::pid_t);
            // 发送 SIGCONT 信号继续执行
            libc::kill(-(pid as libc::pid_t), libc::SIGCONT);
        }
        trace!("恢复 {} 为前台进程组", pid);

        Ok(())
    }

    fn builtin_bg(&mut self, command: &ShellCommand) -> io::Result<()> {
        let index =
            if let Some(arg) = command.arguments.first() {
                // 处理 %n 格式
                if let Some(index_str) = arg.strip_prefix('%') {
                    Some(index_str.parse::<usize>().map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidInput, "无效的作业编号")
                    })?)
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "bg: 参数格式错误，应为 %n",
                    ));
                }
            } else {
                None
            };
        let job = self
            .job_manager
            .bg(index)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "fg: 没有找到该作业"))?;
        println!("{}", job);
        let pid = job.pid;

        #[cfg(unix)]
        unsafe {
            // 发送 SIGCONT 信号在后台继续执行
            libc::kill(-(pid as libc::pid_t), libc::SIGCONT);
        }

        Ok(())
    }
}
