use crate::utils::config::Config;
use chrono::Local;
use env_logger::{Builder, Target};
use log::LevelFilter;
use std::io::Write;
pub fn init_logger(config: &Config) {
    let level = match &config.logger_level {
        level if level.eq_ignore_ascii_case("debug") => LevelFilter::Debug,
        level if level.eq_ignore_ascii_case("trace") => LevelFilter::Trace,
        level if level.eq_ignore_ascii_case("warn") => LevelFilter::Warn,
        level if level.eq_ignore_ascii_case("error") => LevelFilter::Error,
        _ => LevelFilter::Info,
    };

    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}] {} - {}",
                record.level(),
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.args()
            )
        })
        .target(Target::Stdout)
        .filter(Some(&config.name), level)
        .filter(None, LevelFilter::Warn)
        .init();

    log::debug!("日志级别设置为: {}", level);
}
