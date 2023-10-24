use crate::app_config::AppConfig;
use crate::db::{mongo, pgsql};
use anyhow::{Context, Result};
use crossbeam::channel::Receiver;
use tracing::info;

pub fn backup_db(app_config: &AppConfig, shutdown_rx: &Receiver<()>) -> Result<()> {
    let do_postgres = app_config.postgres_backup_enabled.unwrap_or(false);
    let do_mongo = app_config.mongo_backup_enabled.unwrap_or(false);
    let backup_path = app_config.source_path.join("db");

    if do_postgres || do_mongo {
        std::fs::create_dir_all(&backup_path)
            .context("Error while creating top-level database backup directory.")?;
    }

    if do_postgres {
        pgsql::backup_postgres(&backup_path, shutdown_rx)?;
    } else {
        info!("PostgreSQL backup disabled.");
    }

    if do_mongo {
        mongo::backup_mongo(&backup_path, shutdown_rx)?;
    } else {
        info!("MongoDB backup disabled.")
    }

    Ok(())
}
