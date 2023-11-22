use anyhow::Context;
use config::{Config, File};
use parking_lot::RwLock;

lazy_static! {
    pub static ref SETTINGS: RwLock<Config> = RwLock::new(Config::default());
    pub static ref HTTP_WORKERS: usize = http_workers();
    pub static ref JSON_PAYLOAD_LIMIT: usize = json_payload_limit();
}

pub fn settings() -> &'static RwLock<Config> {
    &*SETTINGS
}

fn http_workers() -> usize {
    settings().read().get::<usize>("http_workers").unwrap_or_else(|_| 1)
}

fn json_payload_limit() -> usize {
    settings().read().get::<usize>("json_payload_limit").unwrap_or_else(|_| 1_048_576)
}

pub fn load_global_config(base_dir: &str, env: &str) -> anyhow::Result<()> {
    let mut write_guard = settings().write();

    let mut builder = Config::builder();
    let default_config_file = format!("{}/service-default", base_dir);
    builder = builder.add_source(File::with_name(&default_config_file));

    let env_config_file = format!("{}/service-{}", base_dir, env);
    builder = builder.add_source(File::with_name(&env_config_file));

    let config = builder
        .build()
        .with_context(|| format!("Error in loading config from dir: {} for env: {}", base_dir, env))?;

    *write_guard = config;

    Ok(())
}
