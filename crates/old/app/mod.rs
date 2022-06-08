mod config;

use std::{io::Write, path::Path};

use anyhow::Context;

use crate::app::config::RunConfig;

pub fn run(config: &Path) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(config).context("Failed to open configuration file")?;
    let config =
        toml::from_str::<RunConfig>(&content).context("Failed to read configuration file")?;

    let _dns_resolver = config.dns.make_resolver();

    todo!()
}

pub fn new_config(path: &Path) -> anyhow::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .context("Failed to create file")?;
    file.write_all(include_bytes!("../../data/project_config_template.toml"))?;

    Ok(())
}
