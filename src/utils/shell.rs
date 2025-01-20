use log::{debug, error, warn};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{CompletionType, Config as RLConfig, EditMode, Editor};
use shell_words;
use shellexpand;
use std::env;
use std::error::Error;
use std::io;
use std::io::Write;
use std::process::{Command, Stdio};
use uuid::Uuid;

use crate::utils::config::Config;
use crate::utils::theme::Theme;

pub struct Shell<'a> {
    config: &'a Config,
    id: Uuid,
    theme: Theme,
}

impl<'a> Shell<'a> {
    pub fn new(config: &'a Config) -> Self {
        // 执行主题文件，设置主题环境变量
        let theme_file = Theme::get_theme_file(config);
        match Shell::execute_command(&format!("source {}", theme_file)) {
            Ok(status) => {
                if !status.success() {
                    error!("执行主题文件失败: {}", status);
                } else {
                    debug!("执行主题文件成功：{}", env::var("PROMPT").unwrap_or_default());
                }
            }
            Err(e) => {
                error!("执行主题文件失败: {}", e);
            }
        }
        Self {
            config,
            id: Uuid::new_v4(),
            theme: Theme::new(),
        }
    }

    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        debug!("初始化 ZakoShell... {}", self.id);
        let mut rl = self.setup_readline()?;
        if let Err(err) = rl.load_history(&self.config.history_file) {
            warn!(
                "无法加载历史记录: {} {}",
                self.config.history_file.display(),
                err
            );
        } else {
            debug!("历史记录加载成功");
        }

        println!(
            "{}",
            (self.theme.success_style)(self.theme.get_message("welcome"))
        );
        println!(
            "{}",
            (self.theme.warning_style)(self.theme.get_message("help"))
        );
        debug!("ZakoShell 准备就绪 {}", self.id);

        self.run_loop(&mut rl)?;
        if let Err(err) = rl.save_history(&self.config.history_file) {
            error!("保存历史记录失败: {}", err);
        } else {
            debug!("历史记录保存成功");
        }
        debug!("退出 ZakoShell {}", self.id);
        Ok(())
    }

    fn setup_readline(&self) -> Result<Editor<(), FileHistory>, Box<dyn Error>> {
        let rl_config = RLConfig::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(if self.config.editor_mode == "emacs" {
                EditMode::Emacs
            } else {
                EditMode::Vi
            })
            .build();

        Ok(Editor::with_config(rl_config)?)
    }

    fn run_loop(&self, rl: &mut Editor<(), FileHistory>) -> Result<(), Box<dyn Error>> {
        loop {
            io::stdout().flush()?;
            let prompt = (self.theme.prompt_style)(self.theme.get_message("prompt"));
            match rl.readline(&prompt) {
                Ok(line) => {
                    if line.trim() == "exit" {
                        debug!("退出 ZakoShell");
                        println!(
                            "{}",
                            (self.theme.success_style)(self.theme.get_message("exit"))
                        );
                        break;
                    }
                    self.handle_input(rl, &line)?;
                }
                Err(err) => match err {
                    ReadlineError::Eof => {
                        debug!("接收到 EOF 信号，退出 ZakoShell");
                        println!(
                            "\n{}",
                            (self.theme.warning_style)(self.theme.get_message("eof_signal"))
                        );
                        break;
                    }
                    ReadlineError::Interrupted => {
                        warn!("接收到中断信号");
                        println!(
                            "\n{}",
                            (self.theme.warning_style)(self.theme.get_message("interrupt_signal"))
                        );
                    }
                    err => {
                        error!("发生错误: {}", err);
                        eprintln!(
                            "{}: {}",
                            (self.theme.error_style)(self.theme.get_message("error")),
                            err
                        );
                    }
                },
            }
        }
        Ok(())
    }

    fn handle_input(
        &self,
        rl: &mut Editor<(), FileHistory>,
        line: &str,
    ) -> Result<(), Box<dyn Error>> {
        let args = shell_words::split(&line.trim())?;
        if args.is_empty() {
            return Ok(());
        }

        rl.add_history_entry(line.to_string())?;
        debug!("执行命令: {}", &line);

        // 处理创建子 ZakoShell
        if args[0] == self.config.name {
            let shell = Shell::new(self.config);
            shell.run()?;
            return Ok(());
        }

        // 处理设置环境变量
        if args[0].contains('=') || args[0] == "export" {
            let vars = if args[0] == "export" {
                args[1..].iter()
            } else {
                args[..].iter()
            };

            for var in vars {
                if let Some((name, value)) = var.split_once('=') {
                    let name = name.trim();
                    let value = shellexpand::env(value.trim())?;
                    let value = value.trim_matches('"').trim_matches('\'');
                    env::set_var(name, value);
                }
            }
            return Ok(());
        }

        // 执行普通命令
        match Shell::execute_command(&args.join(" ")) {
            Ok(status) => {
                if status.success() {
                    println!(
                        "{} {}",
                        (self.theme.success_style)(self.theme.get_message("success_symbol")),
                        (self.theme.success_style)(self.theme.get_message("command_success"))
                    );
                } else {
                    eprintln!(
                        "{} {}",
                        (self.theme.error_style)(self.theme.get_message("error_symbol")),
                        (self.theme.error_style)(self.theme.get_message("command_error"))
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "{} {}",
                    (self.theme.error_style)(self.theme.get_message("error_symbol")),
                    (self.theme.error_style)(format!(
                        "{}: {}",
                        self.theme.get_message("execution_error"),
                        e
                    ))
                );
            }
        }
        Ok(())
    }

    fn execute_command(command: &str) -> std::io::Result<std::process::ExitStatus> {
        let (shell, args) = if cfg!(target_os = "windows") {
            ("cmd", ["/C"])
        } else {
            ("sh", ["-c"])
        };

        let mut cmd = Command::new(shell);
        cmd.args([args[0], command])
            .env_clear()
            .envs(env::vars())
            .current_dir(env::current_dir().unwrap_or_default())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
    }
}
