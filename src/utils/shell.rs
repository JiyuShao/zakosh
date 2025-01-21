use log::{debug, error, warn};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{CompletionType, Config as RLConfig, EditMode, Editor};
use shell_words;
use shellexpand;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::utils::config::Config;
use crate::utils::theme::Theme;

pub struct Shell<'a> {
    config: &'a Config,
    theme: Theme,
    local_vars: HashMap<String, String>,
}

impl<'a> Shell<'a> {
    pub fn new(config: &'a Config) -> Self {
        let mut shell = Self {
            config,
            theme: Theme::new(),
            local_vars: HashMap::new(),
        };
        shell.load_theme();
        shell
    }

    fn load_theme(&mut self) -> () {
        let theme_file = Theme::get_theme_file(self.config);

        // 执行主题文件并捕获变量
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                r#"
                # 执行主题文件
                source {}
                
                # 输出环境变量（保持原始格式）
                env | while IFS= read -r line || [ -n "$line" ]; do
                    printf '%s\n' "$line"
                done

                echo "---ENV_VAR_END---"
                
                # 输出所有变量（保持原始格式）
                set | while IFS= read -r line || [ -n "$line" ]; do
                    printf '%s\n' "$line"
                done
                "#,
                theme_file
            ))
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    // println!("{}", String::from_utf8_lossy(&output.stdout));
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let mut env_vars = HashMap::new();
                    let mut all_vars = HashMap::new();
                    let mut current_var = String::new();
                    let mut is_env_section = true;

                    for line in stdout.lines() {
                        if line.is_empty() {
                            continue;
                        }
                        if line == "---ENV_VAR_END---" {
                            is_env_section = false;
                            continue;
                        }

                        // 如果行以字母开头，说明是新变量的开始
                        // TODO: 这里不准确，要做完整的解析器和执行器了
                        if line
                            .chars()
                            .next()
                            .map_or(false, |c| c.is_ascii_alphabetic())
                        {
                            // 处理之前累积的变量（如果有）
                            if !current_var.is_empty() {
                                if let Some((name, value)) =
                                    Self::parse_var_definition(&current_var)
                                {
                                    if is_env_section {
                                        env_vars.insert(name.clone(), value.clone());
                                    }
                                    all_vars.insert(name, value);
                                }
                            }
                            current_var = line.to_string();
                        } else if !current_var.is_empty() {
                            // 继续累积多行变量
                            current_var.push('\n');
                            current_var.push_str(line);
                        }
                    }

                    // 处理最后一个变量
                    if !current_var.is_empty() {
                        if let Some((name, value)) = Self::parse_var_definition(&current_var) {
                            if is_env_section {
                                env_vars.insert(name.clone(), value.clone());
                            }
                            all_vars.insert(name, value);
                        }
                    }

                    // 设置变量
                    for (name, value) in all_vars {
                        if env_vars.contains_key(&name) {
                            debug!("从主题文件加载环境变量: {}={}", name, value);
                            env::set_var(&name, &value);
                        } else {
                            debug!("从主题文件加载全局变量: {}={}", name, value);
                            self.local_vars.insert(name, value);
                        }
                    }

                    debug!("执行主题文件成功");
                } else {
                    error!("执行主题文件失败: {}", output.status);
                    if !output.stderr.is_empty() {
                        error!("错误信息: {}", String::from_utf8_lossy(&output.stderr));
                    }
                }
            }
            Err(e) => {
                error!("执行主题文件失败: {}", e);
            }
        }
    }

    fn parse_var_definition(var_def: &str) -> Option<(String, String)> {
        if let Some((name, value)) = var_def.split_once('=') {
            let name = name.trim().to_string();
            // 处理多行值：保持原始格式，包括换行符
            let value = value
                .trim()
                .to_string();

            Some((name, value.to_string()))
        } else {
            None
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        debug!("初始化 ZakoShell...");
        let mut rl: Editor<(), FileHistory> = self.setup_readline()?;
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
        debug!("ZakoShell 准备就绪...");

        self.run_loop(&mut rl)?;
        if let Err(err) = rl.save_history(&self.config.history_file) {
            error!("保存历史记录失败: {}", err);
        } else {
            debug!("历史记录保存成功");
        }
        debug!("退出 ZakoShell...");
        Ok(())
    }

    fn execute_command(
        command: &str,
        local_vars: &HashMap<String, String>,
    ) -> std::io::Result<std::process::ExitStatus> {
        let (shell, args) = if cfg!(target_os = "windows") {
            ("cmd", ["/C"])
        } else {
            ("sh", ["-c"])
        };

        let mut command_envs = env::vars().collect::<HashMap<String, String>>();
        // 局部变量优先级高于全局变量
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

    fn run_loop(&mut self, rl: &mut Editor<(), FileHistory>) -> Result<(), Box<dyn Error>> {
        loop {
            io::stdout().flush()?;
            let prompt = (self.theme.prompt_style)(self.theme.get_message("prompt"));
            match rl.readline(&prompt) {
                Ok(line) => {
                    if line.trim() == "exit" {
                        debug!("退出 ZakoShell...");
                        println!(
                            "{}",
                            (self.theme.success_style)(self.theme.get_message("exit"))
                        );
                        std::process::exit(0);
                    }
                    self.handle_input(rl, &line)?;
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

    fn handle_input(
        &mut self,
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
            self.run_child_shell();
            return Ok(());
        }

        // 处理设置全局变量、系统环境变量
        // - 全局变量只会在当前 ZakoShell 中生效
        // - 环境变量会设置到系统环境变量中
        if args[0].contains('=') || args[0] == "export" {
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
                        self.local_vars.insert(name, value);
                    }
                }
            }
            return Ok(());
        }

        // 执行普通命令
        match Shell::execute_command(&args.join(" "), &self.local_vars) {
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

    fn run_child_shell(&self) -> () {
        // 获取当前程序的可执行路径
        let current_exe = match env::current_exe() {
            Ok(path) => path,
            Err(err) => {
                eprintln!(
                    "{}: {}",
                    (self.theme.error_style)(self.theme.get_message("error")),
                    err
                );
                return;
            }
        };

        debug!("当前程序的可执行路径: {}", current_exe.display());
        // 创建子进程，并运行当前程序
        let child_process = Command::new(current_exe)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn();

        match child_process {
            Ok(mut child) => {
                if let Err(e) = child.wait() {
                    error!("等待子进程({})失败: {}", child.id(), e);
                } else {
                    debug!("子进程({})执行成功", child.id());
                }
            }
            Err(e) => {
                error!("创建子进程失败: {}", e);
            }
        }
    }
}
