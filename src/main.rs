use log::{debug, error, warn};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{CompletionType, Config as RLConfig, Editor, Result};
use std::io::{self, Write};
use std::process::Command;

use crate::utils::config::Config;
use crate::utils::log::init_logger;
use crate::utils::theme::{load_theme, Theme};

mod utils;

fn main() -> Result<()> {
    let config = Config::new();
    let theme = load_theme(&config.theme);
    init_logger();
    debug!("配置加载成功");

    println!(
        "{}",
        (theme.success_style)(theme.get_message("welcome").clone())
    );
    println!(
        "{}",
        (theme.warning_style)(theme.get_message("help").clone())
    );

    debug!("初始化编辑器...");
    let rl_config = RLConfig::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(config.get_edit_mode())
        .build();

    let mut rl: Editor<(), FileHistory> = Editor::with_config(rl_config)?;

    if let Err(err) = rl.load_history(&config.history_file) {
        warn!("无法加载历史记录: {}", err);
    } else {
        debug!("历史记录加载成功");
    }

    let prompt = theme.prompt.clone();
    debug!("ZakoShell 准备就绪");

    loop {
        io::stdout().flush()?;
        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line)?;
                debug!("执行命令: {}", line);

                if line == "exit" {
                    debug!("退出 ZakoShell");
                    println!(
                        "{}",
                        (theme.success_style)(theme.get_message("exit").clone())
                    );
                    break;
                }

                execute_command(line, &theme);
            }
            Err(ReadlineError::Interrupted) => {
                warn!("接收到中断信号");
                println!("\n{}", theme.get_message("interrupt_signal"));
                continue;
            }
            Err(ReadlineError::Eof) => {
                debug!("接收到 EOF 信号");
                println!("\n{}", theme.get_message("eof_signal"));
                break;
            }
            Err(err) => {
                error!("发生错误: {}", err);
                eprintln!("{}: {}", theme.get_message("error"), err);
                break;
            }
        }
    }

    if let Err(err) = rl.save_history(&config.history_file) {
        error!("保存历史记录失败: {}", err);
    } else {
        debug!("历史记录保存成功");
    }

    Ok(())
}

fn execute_command(command: &str, theme: &Theme) {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", command]).output()
    } else {
        Command::new("sh").args(["-c", command]).output()
    };

    match output {
        Ok(output) => {
            if !output.stdout.is_empty() {
                print!("{}", String::from_utf8_lossy(&output.stdout));
            }
            if !output.stderr.is_empty() {
                eprint!(
                    "{}",
                    (theme.error_style)(String::from_utf8_lossy(&output.stderr).to_string())
                );
            }
            if output.status.success() {
                println!(
                    "{} {}",
                    theme.success_symbol,
                    (theme.success_style)(theme.get_message("command_success").clone())
                );
            } else {
                println!(
                    "{} {}",
                    theme.error_symbol,
                    (theme.error_style)(theme.get_message("command_error").clone())
                );
            }
        }
        Err(e) => {
            eprintln!(
                "{} {}",
                theme.error_symbol,
                (theme.error_style)(format!("{}: {}", theme.get_message("execution_error"), e))
            );
            println!(
                "{}",
                (theme.error_style)(theme.get_message("execution_error").clone())
            );
        }
    }
}
