use log::{debug, error, warn};
use shellexpand;
use shell_words;
use std::env;
use std::error::Error;
use std::io::Write;

use crate::shell::commands::CommandExecutor;
use crate::shell::readline::{ReadlineError, ReadlineManager};
use crate::shell::variables::VariableManager;
use crate::utils::config::Config;
use crate::utils::theme::Theme;

pub struct Shell<'a> {
    config: &'a Config,
    theme: Theme,
    variables: VariableManager,
    readline: ReadlineManager<'a>,
}

impl<'a> Shell<'a> {
    pub fn new(config: &'a Config) -> Self {
        let mut shell = Self {
            config,
            theme: Theme::new(),
            variables: VariableManager::new(),
            readline: ReadlineManager::new(config),
        };
        let theme_file = Theme::get_theme_file(config);
        shell.variables.load_theme_variables(&theme_file);
        shell
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        debug!("初始化 ZakoShell...");
        self.readline.load_history()?;

        println!(
            "{}",
            (self.theme.success_style)(self.theme.get_message("welcome"))
        );
        println!(
            "{}",
            (self.theme.warning_style)(self.theme.get_message("help"))
        );
        debug!("ZakoShell 准备就绪...");

        self.run_loop()?;
        self.readline.save_history()?;

        debug!("退出 ZakoShell...");
        Ok(())
    }

    fn run_loop(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            std::io::stdout().flush()?;
            let prompt = (self.theme.prompt_style)(self.theme.get_message("prompt"));

            match self.readline.readline(&prompt) {
                Ok(line) => {
                    if line.trim() == "exit" {
                        debug!("退出 ZakoShell...");
                        println!(
                            "{}",
                            (self.theme.success_style)(self.theme.get_message("exit"))
                        );
                        std::process::exit(0);
                    }
                    self.handle_input(&line)?;
                }
                Err(err) => match err {
                    ReadlineError::Eof => {
                        warn!("接收到 EOF 信号，退出 ZakoShell...");
                        println!(
                            "\n{}",
                            (self.theme.warning_style)(self.theme.get_message("eof_signal"))
                        );
                        break;
                    }
                    ReadlineError::Interrupted => {
                        warn!("接收到中断信号...");
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

    fn handle_input(&mut self, line: &str) -> Result<(), Box<dyn Error>> {
        let args = shell_words::split(line.trim())?;
        if args.is_empty() {
            return Ok(());
        }

        self.readline.add_history(line.to_string())?;
        debug!("执行命令: {}", &line);

        // 处理创建子 ZakoShell
        if args[0] == self.config.name {
            if let Ok(current_exe) = env::current_exe() {
                if let Err(e) =
                    CommandExecutor::run_child_shell(current_exe.to_str().unwrap_or_default())
                {
                    error!("创建子进程失败: {}", e);
                }
            }
            return Ok(());
        }

        // 处理变量设置
        if args[0].contains('=') || args[0] == "export" {
            self.handle_variable_assignment(&args)?;
            return Ok(());
        }

        // 执行普通命令
        self.execute_command(&args)?;
        Ok(())
    }

    fn handle_variable_assignment(&mut self, args: &[String]) -> Result<(), Box<dyn Error>> {
        let vars = if args[0] == "export" {
            args[1..].iter()
        } else {
            args[..].iter()
        };

        for var in vars {
            if let Some((name, value)) = var.split_once('=') {
                let name = name.trim().to_string();
                let value = shellexpand::env(value.trim())?;
                let value = value.trim_matches('"').trim_matches('\'').to_string();

                let is_system_env = args[0] == "export" || env::var(&name).is_ok();
                if is_system_env {
                    debug!("设置环境变量: {}={}", name, value);
                    env::set_var(&name, &value);
                } else {
                    debug!("设置全局变量: {}={}", name, value);
                    self.variables.set_var(name, value);
                }
            }
        }
        Ok(())
    }

    fn execute_command(&self, args: &[String]) -> Result<(), Box<dyn Error>> {
        match CommandExecutor::execute(&args.join(" "), self.variables.get_vars()) {
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
}
