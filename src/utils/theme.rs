use colored::Colorize;

pub struct Theme {
    pub prompt: String,
    pub success_symbol: String,
    pub error_symbol: String,
    pub welcome_message: String,
    pub exit_message: String,
    pub error_style: Box<dyn Fn(String) -> String>,
    pub success_style: Box<dyn Fn(String) -> String>,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            prompt: "雑魚> ".bright_cyan().to_string(),
            success_symbol: "♡".bright_magenta().to_string(),
            error_symbol: "✗".red().to_string(),
            welcome_message: "哼～又来找人家玩了吗？真是个变态呢～"
                .bright_magenta()
                .to_string(),
            exit_message: "哼！这就走了吗？真是个没用的废物呢！"
                .bright_blue()
                .to_string(),
            error_style: Box::new(|s| s.bright_red().to_string()),
            success_style: Box::new(|s| s.bright_magenta().to_string()),
        }
    }
}

pub fn load_theme(theme_name: &str) -> Theme {
    match theme_name {
        "default" => Theme::default(),
        "dark" => Theme {
            prompt: "雑魚～➤ ".bright_purple().to_string(),
            success_symbol: "♡".bright_magenta().to_string(),
            error_symbol: "✗".red().to_string(),
            welcome_message: "啊啦～这不是变态先生吗？又来了呢～"
                .bright_magenta()
                .to_string(),
            exit_message: "切～这就受不了了吗？真是个废物呢！"
                .bright_purple()
                .to_string(),
            error_style: Box::new(|s| s.red().to_string()),
            success_style: Box::new(|s| s.magenta().to_string()),
        },
        _ => Theme::default(),
    }
}
