use dotenv::dotenv;
use log::error;
use shellexpand;
use std::env;
use std::fs;
use std::path::PathBuf;

pub struct Config {
    pub name: String,
    pub logger_level: String,
    pub theme: String,
    pub editor_mode: String,
    // paths
    pub config_dir: PathBuf,
    pub history_file: PathBuf,
    pub themes_dir: PathBuf,
}

impl Config {
    fn default() -> Self {
        let config_dir = if let Ok(dir) = env::var("ZAKO_CONFIG_DIR") {
            if let Some(stripped) = dir.strip_prefix("./") {
                env::current_dir().unwrap_or_default().join(stripped)
            } else {
                PathBuf::from(shellexpand::tilde(&dir).into_owned())
            }
        } else {
            PathBuf::from(shellexpand::tilde("~/.config/zako").into_owned())
        };
        Config {
            name: String::from("zako"),
            logger_level: String::from("info"),
            theme: String::from("default"),
            editor_mode: String::from("vi"),
            config_dir: config_dir.clone(),
            history_file: config_dir.join(".zako_history"),
            themes_dir: config_dir.join("themes"),
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

        if let Ok(logger_level) = env::var("ZAKO_LOG") {
            config.logger_level = logger_level;
        }

        if let Ok(theme) = env::var("ZAKO_THEME") {
            config.theme = theme;
        }

        if let Ok(editor) = env::var("ZAKO_EDITOR_MODE") {
            config.editor_mode = editor;
        }

        // 确保历史文件目录存在
        if let Some(parent) = config.history_file.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                error!("无法创建历史记录目录: {}", e);
            }
        }

        config
    }
}
