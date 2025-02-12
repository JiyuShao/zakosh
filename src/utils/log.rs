use crate::utils::config::Config;
use chrono::Local;
use env_logger::{Builder, Target};
use log::LevelFilter;
use std::fs::{self, File};
use std::io::Write;
use std::process;

pub fn init_logger(config: &Config) {
    let level = match &config.logger_level {
        level if level.eq_ignore_ascii_case("error") => LevelFilter::Error,
        level if level.eq_ignore_ascii_case("warn") => LevelFilter::Warn,
        level if level.eq_ignore_ascii_case("info") => LevelFilter::Info,
        level if level.eq_ignore_ascii_case("debug") => LevelFilter::Debug,
        level if level.eq_ignore_ascii_case("trace") => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };

    // 创建日志目录
    fs::create_dir_all(&config.logger_dir).expect("Failed to create log directory");
    let date = Local::now().format("%Y-%m-%d");
    let log_file = config
        .logger_dir
        .join(format!("zako_{}.log", date.to_string()));
    let file = File::create(log_file).expect("Failed to create log file");

    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "[PID:{}][{}] {} - {}",
                process::id(),
                record.level(),
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.args()
            )
        })
        .target(Target::Pipe(Box::new(MultiWriter {
            writers: vec![Box::new(std::io::stdout()), Box::new(file)],
        })))
        .filter(Some(&config.name), level)
        .filter(None, LevelFilter::Warn)
        .init();

    log::debug!("日志级别设置为: {}", level);
}

struct MultiWriter {
    writers: Vec<Box<dyn Write + Send + Sync>>,
}

impl Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for writer in &mut self.writers {
            writer.write_all(buf)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        for writer in &mut self.writers {
            writer.flush()?;
        }
        Ok(())
    }
}
