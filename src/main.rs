use colored::Colorize;
use log::{debug, error, warn};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{CompletionType, Config as RLConfig, Editor, Result};
use std::io::{self, Write};
use std::process::Command;

use crate::utils::config::Config;
use crate::utils::theme::{load_theme, Theme};
use crate::utils::log::init_logger;

mod utils;

fn main() -> Result<()> {
    let config = Config::new();
    let theme = load_theme(&config.theme);
    init_logger();
    debug!("配置加载成功");

    println!("{}", theme.welcome_message);
    println!("{}", "输入 'exit' 退出".bright_blue());

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
                    println!("{}", theme.exit_message);
                    break;
                }

                execute_command(line, &theme);
            }
            Err(ReadlineError::Interrupted) => {
                warn!("接收到中断信号");
                println!("\n中断信号 (Ctrl+C)");
                continue;
            }
            Err(ReadlineError::Eof) => {
                debug!("接收到 EOF 信号");
                println!("\n退出信号 (Ctrl+D)");
                break;
            }
            Err(err) => {
                error!("发生错误: {}", err);
                eprintln!("错误: {}", err);
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
                    (theme.success_style)(
                        "哼～勉强算你做对了呢，不过也就这种程度了吧？".to_string()
                    )
                );
            } else {
                println!(
                    "{} {}",
                    theme.error_symbol,
                    (theme.error_style)(
                        "啊啦啊啦～连这么简单的命令都搞不定呢，真是个废物呢！".to_string()
                    )
                );
            }
        }
        Err(e) => {
            eprintln!(
                "{} {}",
                theme.error_symbol,
                (theme.error_style)(format!("笨蛋！命令执行失败了啦: {}", e))
            );
            println!(
                "{}",
                (theme.error_style)("真是个没用的废物呢～这种程度就不行了吗？".to_string())
            );
        }
    }
}
