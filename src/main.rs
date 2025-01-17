use log::{debug, error, warn};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{CompletionType, Config as RLConfig, Editor, EditMode, Result};
use std::io::{self, Write};

use crate::utils::config::CONFIG;
use crate::utils::log::init_logger;
use crate::utils::shell::execute_command;
use crate::utils::theme::load_theme;

mod utils;

fn main() -> Result<()> {
    init_logger();
    let theme = load_theme(&CONFIG.theme);
    debug!("配置加载成功 {}", CONFIG.config_dir.display());

    println!(
        "{}",
        (theme.success_style)(theme.get_message("welcome").clone())
    );
    println!(
        "{}",
        (theme.warning_style)(theme.get_message("help").clone())
    );

    debug!("初始化 ZakoShell...");
    let rl_config = RLConfig::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(if CONFIG.editor_mode == "emacs" {
            EditMode::Emacs
        } else {
            EditMode::Vi
        })
        .build();

    let mut rl: Editor<(), FileHistory> = Editor::with_config(rl_config)?;

    if let Err(err) = rl.load_history(&CONFIG.history_file) {
        warn!(
            "无法加载历史记录: {} {}",
            CONFIG.history_file.display(),
            err
        );
    } else {
        debug!("历史记录加载成功");
    }

    let prompt = (theme.prompt_style)(theme.get_message("prompt").clone());
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

                execute_command(line, &theme, &CONFIG);
            }
            Err(ReadlineError::Interrupted) => {
                warn!("接收到中断信号");
                println!(
                    "\n{}",
                    (theme.warning_style)(theme.get_message("interrupt_signal").clone())
                );
                continue;
            }
            Err(ReadlineError::Eof) => {
                debug!("接收到 EOF 信号");
                println!(
                    "\n{}",
                    (theme.warning_style)(theme.get_message("eof_signal").clone())
                );
                break;
            }
            Err(err) => {
                error!("发生错误: {}", err);
                eprintln!(
                    "{}: {}",
                    (theme.error_style)(theme.get_message("error").clone()),
                    err
                );
                break;
            }
        }
    }

    if let Err(err) = rl.save_history(&CONFIG.history_file) {
        error!("保存历史记录失败: {}", err);
    } else {
        debug!("历史记录保存成功");
    }

    Ok(())
}
