#[macro_use]
extern crate lazy_static;

use crate::shell::Shell;
use log::debug;

use crate::utils::config::Config;
use crate::utils::log::init_logger;

mod shell;
mod utils;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new();
    init_logger(&config);
    debug!("配置加载成功 {}", config.config_dir.display());

    let mut shell = Shell::new(&config);
    shell.run()
}
