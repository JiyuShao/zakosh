use std::collections::HashMap;
use std::env;
use std::process::{Command, Stdio};

use crate::utils::config::Config;
use crate::utils::theme::Theme;

pub fn execute_command(command: &str, theme: &Theme, config: &Config) {
    let args: Vec<&str> = command.split_whitespace().collect();
    if args.is_empty() {
        return;
    }

    // 检查是否是 zako 命令
    if args[0] == config.name {
        // 创建一个新的干净环境
        let mut clean_env = HashMap::new();
        // 只保留必要的环境变量
        clean_env.insert("TERM", env::var("TERM").unwrap_or_default());
        clean_env.insert("PATH", env::var("PATH").unwrap_or_default());
        clean_env.insert("HOME", env::var("HOME").unwrap_or_default());
        clean_env.insert("USER", env::var("USER").unwrap_or_default());
        clean_env.insert(
            "SHELL",
            format!("/bin/{}", config.name),
        );

        // 启动新的 shell 实例，使用清理过的环境
        let status = create_command("")
            .env_clear()
            .envs(&clean_env)
            .current_dir(env::current_dir().unwrap_or_default())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

        handle_command_result(status, theme);
        return;
    }

    // 处理其他普通命令，保持当前环境

    let command_result = create_command(command)
        .envs(env::vars())
        .current_dir(env::current_dir().unwrap_or_default())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    handle_command_result(command_result, theme);
}

// 根据操作系统选择合适的 shell 命令
fn create_command(command: &str) -> Command {
    let (shell, args) = if cfg!(target_os = "windows") {
        ("cmd", ["/C"])
    } else {
        ("sh", ["-c"])
    };
    
    let mut cmd = Command::new(shell);
    cmd.args([args[0], command]);
    cmd
}

// 抽取公共的结果处理逻辑
fn handle_command_result(result: std::io::Result<std::process::ExitStatus>, theme: &Theme) {
    match result {
        Ok(status) => {
            if status.success() {
                println!(
                    "{} {}",
                    (theme.success_style)(theme.get_message("success_symbol").clone()),
                    (theme.success_style)(theme.get_message("command_success").clone())
                );
            } else {
                eprintln!(
                    "{} {}",
                    (theme.error_style)(theme.get_message("error_symbol").clone()),
                    (theme.error_style)(theme.get_message("command_error").clone())
                );
            }
        }
        Err(e) => {
            eprintln!(
                "{} {}",
                (theme.error_style)(theme.get_message("error_symbol").clone()),
                (theme.error_style)(format!("{}: {}", theme.get_message("execution_error"), e))
            );
        }
    }
}
