use log::{debug, error, warn};
use std::error::Error;
use std::io::Write;
use std::sync::Arc;

use crate::shell::executor::Executor;
use crate::shell::job_manager::JobManager;
use crate::shell::parser::Parser;
use crate::shell::readline::{ReadlineError, ReadlineManager};
use crate::shell::scheduler::Scheduler;
use crate::utils::config::Config;
use crate::utils::theme::Theme;

pub struct Shell<'a> {
    theme: Theme,
    readline: ReadlineManager<'a>,
    executor: Executor,
    scheduler: Arc<Scheduler>,
}

impl<'a> Shell<'a> {
    pub fn new(config: &'a Config) -> Self {
        let scheduler = Arc::new(Scheduler::new());
        let job_manager = Arc::new(JobManager::new());
        Self {
            theme: Theme::new(),
            readline: ReadlineManager::new(config),
            executor: Executor::new(Arc::clone(&scheduler), Arc::clone(&job_manager)),
            scheduler,
        }
        // let theme_file = Theme::get_theme_file(config);
        // shell.variables.load_theme_variables(&theme_file);
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

            // 等待直到 shell 成为前台进程组
            self.scheduler.wait_until_foreground();

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
        if line.trim().is_empty() {
            return Ok(());
        }

        self.readline.add_history(line.to_string())?;
        // 使用 parser 解析命令
        let mut parser = Parser::new(line);
        match parser.parse_command() {
            Ok(node) => match self.executor.execute(node) {
                Ok(_) => {
                    println!(
                        "{} {}",
                        (self.theme.success_style)(self.theme.get_message("success_symbol")),
                        (self.theme.success_style)(self.theme.get_message("command_success"))
                    );
                }
                Err(e) => {
                    println!("{}", e);
                    eprintln!(
                        "{} {}",
                        (self.theme.error_style)(self.theme.get_message("error_symbol")),
                        (self.theme.error_style)(self.theme.get_message("command_error")),
                    );
                }
            },
            Err(e) => {
                println!("{}", e);
                eprintln!(
                    "{} {}",
                    (self.theme.error_style)(self.theme.get_message("error_symbol")),
                    (self.theme.error_style)(self.theme.get_message("command_error")),
                );
            }
        }
        Ok(())
    }

    // fn handle_variable_assignment(&mut self, args: &[String]) -> Result<(), Box<dyn Error>> {
    //     let vars = if args[0] == "export" {
    //         args[1..].iter()
    //     } else {
    //         args[..].iter()
    //     };

    //     for var in vars {
    //         if let Some((name, value)) = var.split_once('=') {
    //             let name = name.trim().to_string();
    //             let value = shellexpand::env(value.trim())?;
    //             let value = value.trim_matches('"').trim_matches('\'').to_string();

    //             let is_system_env = args[0] == "export" || env::var(&name).is_ok();
    //             if is_system_env {
    //                 debug!("设置环境变量: {}={}", name, value);
    //                 env::set_var(&name, &value);
    //             } else {
    //                 debug!("设置全局变量: {}={}", name, value);
    //                 self.variables.set_var(name, value);
    //             }
    //         }
    //     }
    //     Ok(())
    // }
}
