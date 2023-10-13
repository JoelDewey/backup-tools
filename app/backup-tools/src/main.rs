use crate::app_config::AppConfig;
use crate::file::backup_files;
use crate::pgsql::backup::backup_postgres;
use anyhow::Result;
use crossbeam::channel::unbounded;
use envy::from_env;
use tracing::info;

mod app_config;
mod common;
mod file;
mod k8s;
mod pgsql;

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();
    info!("Beginning backup process...");

    let (tx, rx) = unbounded();
    ctrlc::set_handler(move || tx.send(()).expect("Failed to send signal on channel."))?;

    let app_config = from_env::<AppConfig>()?;

    let scale_deployment_enabled = app_config.scale_deployment_enabled.unwrap_or(false);
    k8s::scale::scale_deployment(scale_deployment_enabled, || {
        backup_postgres(&app_config, &rx)?;
        backup_files(&app_config, &rx)?;

        Ok(())
    })?;

    info!("Backup completed!");
    Ok(())
}
