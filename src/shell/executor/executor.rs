use log::debug;
use std::{env, io, process};

use crate::shell::parser::ast::{Command as ShellCommand, Node};

use super::variable::Variable;

pub struct Executor {
    variables: Variable,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            variables: Variable::new(),
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
        let (shell, args) = if cfg!(target_os = "windows") {
            ("cmd", ["/C"])
        } else {
            ("sh", ["-c"])
        };
        let program = format!(
            "{} {}",
            command.program,
            command
                .arguments
                .iter()
                .map(|arg| self.expand_variables(arg))
                .collect::<Vec<String>>()
                .join(" ")
        )
        .trim()
        .to_owned();
        debug!("解析后的外部命令: {:?}", program);
        let status = process::Command::new(shell)
            .args([args[0], &program])
            .env_clear()
            .envs(self.variables.get_all())
            .current_dir(env::current_dir().unwrap_or_default())
            .stdin(process::Stdio::inherit())
            .stdout(process::Stdio::inherit())
            .stderr(process::Stdio::inherit())
            .status();

        match status {
            Ok(status) => {
                if status.success() {
                    Ok(())
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Exit Code: {}", status.code().unwrap_or(1)),
                    ))
                }
            }
            Err(e) => Err(e),
        }
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
            "zako" => Some(self.buildin_shell()),
            "cd" => Some(self.builtin_cd(command)),
            "exit" => Some(self.builtin_exit()),
            "set" => Some(self.builtin_set(command)),
            _ => None,
        }
    }

    fn buildin_shell(&self) -> io::Result<()> {
        let executable = env::current_exe().unwrap_or_default();
        let _ = process::Command::new(executable)
            .stdout(process::Stdio::inherit())
            .stderr(process::Stdio::inherit())
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
}
