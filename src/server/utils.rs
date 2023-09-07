use std::path::Path;
use anyhow::Context;

use crate::server::settings;

pub fn load_config(config_dir: &str, env: &str) -> anyhow::Result<()> {
    settings::load_global_config(config_dir, env)
        .with_context(|| format!("Error in loading config from dir: {}", config_dir))?;
    return Ok(());
}

pub fn setup_logging(log4rs_file: &str) -> anyhow::Result<()> {
    log4rs::init_file(Path::new(log4rs_file), Default::default())
        .with_context(|| format!("Error in opening log file: {}", log4rs_file))?;

    return Ok(());
}