use crate::utils::config::Config;
use log::{debug, error, warn};
pub use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::Editor;
use rustyline::{CompletionType, Config as RLConfig, EditMode};

pub struct ReadlineManager<'a> {
    config: &'a Config,
    editor: Editor<(), FileHistory>,
}

impl<'a> ReadlineManager<'a> {
    pub fn new(config: &'a Config) -> Self {
        let rl_config = RLConfig::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(if config.editor_mode == "emacs" {
                EditMode::Emacs
            } else {
                EditMode::Vi
            })
            .build();

        let editor = Editor::with_config(rl_config).unwrap_or_else(|err| {
            error!("无法初始化 readline: {}", err);
            panic!("无法初始化 readline");
        });
        Self { config, editor }
    }

    pub fn load_history(&mut self) -> Result<(), ReadlineError> {
        if let Err(err) = self.editor.load_history(&self.config.history_file) {
            warn!(
                "无法加载历史记录: {} {}",
                self.config.history_file.display(),
                err
            );
        } else {
            debug!("历史记录加载成功");
        }
        Ok(())
    }

    pub fn readline(&mut self, prompt: &str) -> Result<String, ReadlineError> {
        self.editor.readline(prompt)
    }

    pub fn add_history(&mut self, line: String) -> Result<bool, ReadlineError> {
        self.editor.add_history_entry(line)
    }

    pub fn save_history(&mut self) -> Result<(), ReadlineError> {
        if let Err(err) = self.editor.save_history(&self.config.history_file) {
            error!("保存历史记录失败: {}", err);
        } else {
            debug!("历史记录保存成功");
        }
        Ok(())
    }
}
