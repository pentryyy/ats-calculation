mod config;
mod dto;
mod server;
mod services;
mod utils;

use crate::config::config::AppConfig;

use crate::server::server::run;
use anyhow::Result;

fn main() -> Result<()> {
    let cfg = AppConfig::load()?;

    if run(&cfg).is_err() {
        std::process::exit(1);
    }

    Ok(())
}
