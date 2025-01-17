use log::{debug, error, warn};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{CompletionType, Config as RLConfig, EditMode, Editor};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io;
use std::io::Write;
use std::process::{Command, Stdio};
use uuid::Uuid;

use crate::utils::config::Config;
use crate::utils::theme::Theme;

pub struct Shell<'a> {
    id: Uuid,
    theme: &'a Theme,
    config: &'a Config,
    envs: HashMap<&'a str, String>,
}

impl<'a> Shell<'a> {
    pub fn new(config: &'a Config, theme: &'a Theme) -> Self {
        let mut envs = HashMap::new();
        envs.insert("TERM", env::var("TERM").unwrap_or_default());
        envs.insert("PATH", env::var("PATH").unwrap_or_default());
        envs.insert("HOME", env::var("HOME").unwrap_or_default());
        envs.insert("USER", env::var("USER").unwrap_or_default());
        envs.insert("SHELL", format!("/bin/{}", config.name));

        Self {
            id: Uuid::new_v4(),
            theme,
            config,
            envs,
        }
    }

    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        println!(
            "{}",
            (self.theme.success_style)(self.theme.get_message("welcome"))
        );
        println!(
            "{}",
            (self.theme.warning_style)(self.theme.get_message("help"))
        );
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

        let prompt = (self.theme.prompt_style)(self.theme.get_message("prompt"));
        debug!("ZakoShell 准备就绪 {}", self.id);

        self.run_loop(&mut rl, &prompt)?;
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

    fn run_loop(
        &self,
        rl: &mut Editor<(), FileHistory>,
        prompt: &str,
    ) -> Result<(), Box<dyn Error>> {
        loop {
            io::stdout().flush()?;
            match rl.readline(prompt) {
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
        let line = line.trim();
        let args: Vec<&str> = line.split_whitespace().collect();
        if args.is_empty() {
            return Ok(true);
        }

        rl.add_history_entry(line)?;
        debug!("执行命令: {}", line);

        if args[0] == self.config.name {
            let shell = Shell::new(self.config, self.theme);
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
            self.execute_command(&args);
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

    fn execute_command(&self, args: &Vec<&str>) {
        let command_result = self
            .create_command(&args.join(" "))
            .env_clear()
            .envs(self.envs.clone())
            .current_dir(env::current_dir().unwrap_or_default())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

        self.handle_command_result(command_result);
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
