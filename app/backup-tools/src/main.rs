use crate::app_config::AppConfig;
use crate::db::backup_db;
use crate::file::backup_files;
use anyhow::Result;
use crossbeam::channel::{unbounded, Receiver};
use envy::from_env;
use rustls::crypto;
use tracing::info;

mod app_config;
mod common;
mod db;
mod file;
mod k8s;

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    crypto::aws_lc_rs::default_provider().install_default().expect("Failed to install rustls crypto provider.");
    
    info!("Beginning backup process...");

    let (tx, rx) = unbounded();
    ctrlc::set_handler(move || tx.send(()).expect("Failed to send signal on channel."))?;

    let app_config = from_env::<AppConfig>()?;

    let scale_deployment_enabled = app_config.scale_deployment_enabled.unwrap_or(false);
    if scale_deployment_enabled {
        k8s::scale::scale_deployment(|| run_backup(&app_config, &rx))?;
    } else {
        info!("Deployment scaling disabled, executing backup immediately.");
        run_backup(&app_config, &rx)?;
    }

    info!("Backup completed!");
    Ok(())
}

fn run_backup(app_config: &AppConfig, shutdown_rx: &Receiver<()>) -> Result<()> {
    backup_db(app_config, shutdown_rx)?;
    backup_files(app_config, shutdown_rx)?;

    Ok(())
}
