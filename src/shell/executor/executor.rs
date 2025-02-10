use log::debug;
use std::collections::HashMap;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::{env, io};

use crate::shell::parser::ast::{Command as ShellCommand, Node};

use super::variable::Variable;

#[derive(Debug)]
pub enum JobStatus {
    Running,
    Stopped,
}

#[derive(Debug)]
pub struct Job {
    command: String,
    status: JobStatus,
}

pub struct Executor {
    variables: Variable,
    jobs: HashMap<u32, Job>,
}

impl Executor {
    pub fn new() -> Self {
        #[cfg(unix)]
        unsafe {
            // shell 进程忽略这些信号
            libc::signal(libc::SIGINT, libc::SIG_IGN); // Ctrl-C
            libc::signal(libc::SIGQUIT, libc::SIG_IGN); // Ctrl-\
            libc::signal(libc::SIGTSTP, libc::SIG_IGN); // Ctrl-Z
            libc::signal(libc::SIGTTOU, libc::SIG_IGN); // 当后台进程尝试写入终端时不暂停进程
            libc::signal(libc::SIGTTIN, libc::SIG_IGN); // 当后台进程尝试从终端读取输入时不暂停进程
        }

        Self {
            variables: Variable::new(),
            jobs: HashMap::new(),
        }
    }

    pub fn execute(&mut self, node: Node) -> io::Result<()> {
        match node {
            Node::Pipeline(pipeline) => self.execute_pipeline(pipeline),
            Node::Command(command) => self.execute_command(command),
        }
    }

    // fn sigtstp_handler(&mut self) {
    //     unsafe {
    //         // 获取当前前台进程组
    //         let foreground_pgid = libc::tcgetpgrp(libc::STDIN_FILENO);
    //         if foreground_pgid == -1 {
    //             eprintln!("Error getting foreground process group");
    //             return;
    //         }

    //         // 暂停当前前台进程组
    //         libc::kill(-foreground_pgid, libc::SIGTSTP);

    //         // 查询并更改当前 job 状态
    //         let job = self.jobs.get_mut(&foreground_pgid).unwrap();
    //         job.status = JobStatus::Stopped;
    //     }
    // }

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
        let args: Vec<String> = command
            .arguments
            .iter()
            .map(|arg| self.expand_variables(arg))
            .collect();
        debug!("解析后的外部命令: {} {:?}", program, args);

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

                // 在子进程启动时就设置进程组
                let pid = libc::getpid();
                libc::setpgid(pid, pid);

                Ok(())
            });
        }

        let child = command
            .args(args)
            .env_clear()
            .envs(self.variables.get_all())
            .current_dir(env::current_dir().unwrap_or_default())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        let pid = child.id();
        self.jobs.insert(
            pid,
            Job {
                command: program.clone(),
                status: JobStatus::Running,
            },
        );

        #[cfg(unix)]
        unsafe {
            // 为子进程单独设置进程组
            libc::setpgid(pid as libc::pid_t, pid as libc::pid_t);

            // 将子进程设为前台进程组
            let shell_terminal = libc::STDIN_FILENO;
            libc::tcsetpgrp(shell_terminal, pid as libc::pid_t);
        }

        // 等待子进程，捕获暂停事件
        let mut result: Result<(), io::Error> = Ok(());
        #[cfg(unix)]
        unsafe {
            let mut status = 0;
            loop {
                match libc::waitpid(pid as libc::pid_t, &mut status, libc::WUNTRACED) {
                    -1 => {
                        result = Err(io::Error::last_os_error());
                        break;
                    }
                    _ => {
                        if libc::WIFSTOPPED(status) {
                            // 子进程被暂停（SIGTSTP）
                            if let Some(job) = self.jobs.get_mut(&pid) {
                                job.status = JobStatus::Stopped;
                                // 打印提示信息
                                println!("\n[{}] Stopped\t{}", pid, job.command);
                            }
                            break;
                        } else if libc::WIFEXITED(status) {
                            // 子进程正常退出
                            self.jobs.remove(&pid);
                            break;
                        } else if libc::WIFSIGNALED(status) {
                            // 子进程被信号终止
                            self.jobs.remove(&pid);
                            // 如果是被信号终止，打印一个换行，因为信号处理可能打断了输出
                            println!();
                            break;
                        }
                    }
                }
            }

            // 恢复 shell 为前台进程组
            let shell_terminal = libc::STDIN_FILENO;
            let shell_pgid = libc::getpid();
            libc::tcsetpgrp(shell_terminal, shell_pgid);
        }

        result
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
        for (pid, job) in &self.jobs {
            let status = match job.status {
                JobStatus::Running => "Running",
                JobStatus::Stopped => "Stopped",
            };
            println!("[{}] {} \t{}", pid, status, job.command);
        }
        Ok(())
    }

    fn builtin_fg(&mut self, command: &ShellCommand) -> io::Result<()> {
        let pid = command
            .arguments
            .first()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "需要指定作业 ID"))?
            .parse::<u32>()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "无效的作业 ID"))?;

        if let Some(job) = self.jobs.get_mut(&pid) {
            // 添加：更新作业状态
            job.status = JobStatus::Running;

            #[cfg(unix)]
            unsafe {
                // 将作业设置为前台进程组
                libc::tcsetpgrp(0, pid as libc::pid_t);
                // 发送 SIGCONT 信号继续执行
                libc::kill(-(pid as libc::pid_t), libc::SIGCONT);

                // 添加：等待前台进程完成或停止
                let mut status: libc::c_int = 0;
                libc::waitpid(-(pid as libc::pid_t), &mut status, libc::WUNTRACED);

                // 恢复 shell 为前台进程组
                libc::tcsetpgrp(0, libc::getpid());

                // 更新作业状态
                if libc::WIFSTOPPED(status) {
                    job.status = JobStatus::Stopped;
                } else {
                    self.jobs.remove(&pid);
                }
            }
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "没有找到该作业"))
        }
    }

    fn builtin_bg(&mut self, command: &ShellCommand) -> io::Result<()> {
        let pid = command
            .arguments
            .first()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "需要指定作业 ID"))?
            .parse::<u32>()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "无效的作业 ID"))?;

        if let Some(job) = self.jobs.get_mut(&pid) {
            #[cfg(unix)]
            unsafe {
                // 发送 SIGCONT 信号在后台继续执行
                libc::kill(-(pid as libc::pid_t), libc::SIGCONT);
            }
            job.status = JobStatus::Running;
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "No such job"))
        }
    }
}
