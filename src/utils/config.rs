use dotenv::dotenv;
use rustyline::EditMode;
use std::env;
use std::fs;
use std::path::PathBuf;

pub struct Config {
    pub theme: String,
    pub history_file: PathBuf,
    pub editor_mode: String,
}

impl Config {
    fn get_config_dir() -> PathBuf {
        if let Ok(home) = env::var("HOME") {
            PathBuf::from(home).join(".config/zako")
        } else {
            PathBuf::from("tmp")
        }
    }

    fn default() -> Self {
        let config_dir = Self::get_config_dir();
        Config {
            theme: String::from("default"),
            history_file: config_dir.join(".zako_history"),
            editor_mode: String::from("vi"),
        }
    }

    pub fn new() -> Self {
        // 优先加载环境变量
        if cfg!(debug_assertions) {
            dotenv::from_filename(".env.development").ok();
        } else {
            dotenv().ok();
        }

        // 默认配置
        let mut config = Config::default();

        // 从环境变量加载配置
        if let Ok(theme) = env::var("ZAKO_THEME") {
            config.theme = theme;
        }

        if let Ok(editor) = env::var("ZAKO_EDITOR") {
            config.editor_mode = editor;
        }

        if let Ok(history) = env::var("ZAKO_HISTORY") {
            config.history_file = PathBuf::from(history);
        }

        // 确保历史文件目录存在
        if let Some(parent) = config.history_file.parent() {
            fs::create_dir_all(parent).expect("无法创建历史记录目录");
        }

        config
    }

    pub fn get_edit_mode(&self) -> EditMode {
        match self.editor_mode.to_lowercase().as_str() {
            "emacs" => EditMode::Emacs,
            _ => EditMode::Vi,
        }
    }
}
