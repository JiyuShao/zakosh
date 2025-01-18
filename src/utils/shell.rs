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
        Self {
            config,
            id: Uuid::new_v4(),
            theme: Theme::load_theme(config),
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
                    if !self.handle_input(rl, &line)? {
                        break;
                    }
                }
                Err(err) => {
                    if !self.handle_readline_error(err) {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_input(
        &self,
        rl: &mut Editor<(), FileHistory>,
        line: &str,
    ) -> Result<bool, Box<dyn Error>> {
        let line = shellexpand::env(line.trim())?;
        let args = shell_words::split(&line)?;
        if args.is_empty() {
            return Ok(true);
        }

        rl.add_history_entry(line.to_string())?;
        debug!("执行命令: {}", &line);

        if args[0] == self.config.name {
            let shell = Shell::new(self.config);
            shell.run()?;
            Ok(true)
        } else if args[0] == "exit" {
            debug!("退出 ZakoShell");
            println!(
                "{}",
                (self.theme.success_style)(self.theme.get_message("exit"))
            );
            Ok(false)
        } else {
            self.execute_command(&args)?;
            Ok(true)
        }
    }

    fn handle_readline_error(&self, err: ReadlineError) -> bool {
        match err {
            ReadlineError::Interrupted => {
                warn!("接收到中断信号");
                println!(
                    "\n{}",
                    (self.theme.warning_style)(self.theme.get_message("interrupt_signal"))
                );
                true
            }
            ReadlineError::Eof => {
                debug!("接收到 EOF 信号");
                println!(
                    "\n{}",
                    (self.theme.warning_style)(self.theme.get_message("eof_signal"))
                );
                false
            }
            err => {
                error!("发生错误: {}", err);
                eprintln!(
                    "{}: {}",
                    (self.theme.error_style)(self.theme.get_message("error")),
                    err
                );
                false
            }
        }
    }

    fn execute_command(&self, args: &[String]) -> Result<(), Box<dyn Error>> {
        // 处理环境变量赋值
        if args.len() == 1 && (args[0].contains('=') || args[0].starts_with("export")) {
            let input = &args[0];
            let assignment = if input.starts_with("export ") {
                input.trim_start_matches("export ").trim()
            } else {
                input
            };

            if let Some((name, value)) = assignment.split_once('=') {
                let name = name.trim();
                let value = shellexpand::env(value.trim())?;
                let value = value.trim_matches('"').trim_matches('\'');
                env::set_var(name, value);
                return Ok(());
            }
        }

        let command_result = self
            .create_command(&args.join(" "))
            .env_clear()
            .envs(env::vars())
            .current_dir(env::current_dir().unwrap_or_default())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

        self.handle_command_result(command_result);
        Ok(())
    }

    fn create_command(&self, command: &str) -> Command {
        let (shell, args) = if cfg!(target_os = "windows") {
            ("cmd", ["/C"])
        } else {
            ("sh", ["-c"])
        };

        let mut cmd = Command::new(shell);
        cmd.args([args[0], command]);
        cmd
    }

    fn handle_command_result(&self, result: std::io::Result<std::process::ExitStatus>) {
        match result {
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
    }
}
