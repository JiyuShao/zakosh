use std::process::{Command, Stdio};
use std::collections::HashMap;
use std::env;

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn execute(command: &str, local_vars: &HashMap<String, String>) -> std::io::Result<std::process::ExitStatus> {
        let (shell, args) = if cfg!(target_os = "windows") {
            ("cmd", ["/C"])
        } else {
            ("sh", ["-c"])
        };

        let mut command_envs = env::vars().collect::<HashMap<String, String>>();
        command_envs.extend(local_vars.clone());

        Command::new(shell)
            .args([args[0], command])
            .env_clear()
            .envs(command_envs)
            .current_dir(env::current_dir().unwrap_or_default())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
    }

    pub fn run_child_shell(executable: &str) -> std::io::Result<std::process::ExitStatus> {
        Command::new(executable)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
            .wait()
    }
} 