use log::{debug, error, trace};
use std::ffi::CString;
#[cfg(unix)]
use std::process::{self, Command, Stdio};
use std::{env, io};

use super::variable::Variable;
use crate::shell::job_manager::JobManager;
use crate::shell::parser::ast::{Command as ShellCommand, Node};
use crate::shell::signals;
use crate::utils::path;

pub struct Executor {
    variables: Variable,
    job_manager: JobManager,
}

impl Executor {
    pub fn new(job_manager: JobManager) -> Self {
        Self {
            variables: Variable::new(),
            job_manager,
        }
    }

    pub fn execute(&mut self, node: Node) -> io::Result<()> {
        let mut pgid: i32 = 0;
        let mut fg_pids: Vec<i32> = Vec::new();
        let _ = match node {
            Node::Pipeline(pipeline) => self.execute_pipeline(pipeline, &mut pgid, &mut fg_pids),
            Node::Command(command) => self.execute_command(command, &mut pgid, &mut fg_pids),
        };

        // 等待 fg 命令执行完毕
        self.job_manager.wait_fg_job(pgid, &fg_pids);

        unsafe {
            let gid = libc::getpgid(0);
            signals::give_terminal_to(gid);
        }

        Ok(())
    }

    fn execute_pipeline(
        &mut self,
        pipeline: Vec<ShellCommand>,
        pgid: &mut i32,
        fg_pids: &mut Vec<i32>,
    ) -> io::Result<()> {
        // 暂时只处理单个命令，后续可以扩展管道功能
        if let Some(command) = pipeline.first() {
            self.execute_command(command.clone(), pgid, fg_pids)
        } else {
            Ok(())
        }
    }

    fn execute_command(
        &mut self,
        command: ShellCommand,
        pgid: &mut i32,
        fg_pids: &mut Vec<i32>,
    ) -> io::Result<()> {
        // // 处理内建命令
        // if let Some(result) = self.handle_builtin(&command) {
        //     debug!("执行内建命令: {:?}", command);
        //     return result;
        // }

        // 执行外部命令
        debug!("执行外部命令: {:?}", command);
        let program = command.program;
        let original_args = command.arguments.clone();
        let args: Vec<String> = command
            .arguments
            .iter()
            .map(|arg| self.expand_variables(arg))
            .collect();

        match unsafe { nix::unistd::fork() } {
            Ok(nix::unistd::ForkResult::Parent { child }) => {
                // 父进程
                let child_pid: i32 = child.into();
                *pgid = child_pid;
                fg_pids.push(child_pid.clone());

                unsafe {
                    // we need to wait pgid of child set to itself,
                    // before give terminal to it (for macos).
                    // 1. this loop causes `bash`, `htop` etc to go `T` status
                    //    immediate after start on linux (ubuntu).
                    // 2. but on mac, we need this loop, otherwise commands
                    //    like `vim` will go to `T` status after start.
                    if cfg!(target_os = "macos") {
                        loop {
                            let _pgid = libc::getpgid(child_pid);
                            if _pgid == child_pid {
                                break;
                            }
                        }
                    }
                }

                signals::give_terminal_to(child_pid);

                self.job_manager.add_job(
                    child_pid,
                    child_pid,
                    program.clone()
                        + " "
                        + original_args
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                            .as_str(),
                );
            }
            Ok(nix::unistd::ForkResult::Child) => {
                // 子进程
                // 恢复子 shell 的 block 信号处理
                signals::notice_block_signals();

                // 设置子进程的进程组
                let pid = unsafe {
                    let pid = libc::getpid();
                    libc::setpgid(0, pid);
                    pid
                };

                // 执行内建命令
                // if cmd.is_builtin() {
                //     trace!("运行内建命令[{}]: {} {:?}", pid, program, args);
                //     if let Some(status) = try_run_builtin_in_subprocess(sh, cl, idx_cmd, capture) {
                //         process::exit(status);
                //     }
                // }

                // 执行外部命令
                let program_path = path::find_file_in_path(program.as_str(), true);
                trace!("运行外部命令[{}]: {} {:?}", pid, program_path, args);
                let c_program = CString::new(program_path).expect("CString::new failed");
                let mut c_args = vec![c_program.clone()]; // 添加程序名作为第一个参数
                c_args.extend(
                    args.iter()
                        .map(|s| CString::new(s.as_str()).expect("CString::new failed")),
                );
                let c_envs = self
                    .variables
                    .get_all()
                    .iter()
                    .map(|(k, v)| {
                        CString::new(format!("{}={}", k, v)).expect("CString::new failed")
                    })
                    .collect::<Vec<_>>();

                match nix::unistd::execve(&c_program, &c_args, &c_envs) {
                    Ok(_) => {}
                    Err(e) => match e {
                        nix::Error::ENOEXEC => {
                            error!("zako: {}: exec format error (ENOEXEC)", program);
                        }
                        nix::Error::ENOENT => {
                            error!("zako: {}: file does not exist", program);
                        }
                        nix::Error::EACCES => {
                            error!("zako: {}: Permission denied", program);
                        }
                        _ => {
                            error!("zako: {}: {:?}", program, e);
                        }
                    },
                }

                process::exit(1);
            }
            Err(e) => {
                error!("Fork failed: {}", e);
                return Err(io::Error::new(io::ErrorKind::Other, "Fork failed"));
            }
        }

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
        for job in self.job_manager.get_jobs() {
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
