use std::fs::create_dir_all;
use crate::common::process::wait_for_subprocess;
use crate::db::mongo::config;
use crate::db::mongo::config::MongoConfig;
use anyhow::{anyhow, Context, Result};
use crossbeam::channel::Receiver;
use envy::prefixed;
use std::path::Path;
use subprocess::{Popen, Redirection};
use tracing::{debug, info, trace_span};
use url::Url;

pub fn backup_mongo(base_backup_path: &Path, shutdown_rx: &Receiver<()>) -> Result<()> {
    let span = trace_span!("mongo");
    let _ = span.enter();

    info!("Starting MongoDB backup.");
    let config = get_mongo_config()?;
    let db_name = config
        .database_name
        .as_ref()
        .map(|s| s as &str)
        .unwrap_or_else(|| "mongodb");
    let backup_path = base_backup_path.join(format!("mongo/{}", db_name));
    create_dir_all(&backup_path).context("Failed to create backup directory during MongoDB backup.")?;

    start_backup(&config, &backup_path, shutdown_rx)?;

    Ok(())
}

fn get_mongo_config() -> Result<MongoConfig> {
    prefixed(config::MONGO_PREFIX)
        .from_env()
        .map_err(|e| anyhow!(e))
        .context("Error while mapping MongoConfig from individual env vars.")
}

fn execute_mongodump(config: &MongoConfig, save_path: &Path) -> Result<Popen> {
    let port = &config.port.unwrap_or(config::DEFAULT_PORT);
    let connection_string = Url::parse(&format!("mongodb://{}:{}", &config.host, port))
        .context("Error encountered while creating MongoDB connection string.")?;

    let mut process = subprocess::Exec::cmd("mongodump")
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Pipe)
        .cwd(save_path)
        .arg("--config")
        .arg(config.configuration_file.as_os_str())
        .arg("--username")
        .arg(&config.username)
        .arg("--gzip")
        .arg("--archive=mongo.gz");

    if let Some(db) = &config.database_name {
        process = process.arg("--db").arg(db).arg("--dumpDbUsersAndRoles");
    }

    if let Some(adb) = &config.authentication_database_name {
        process = process.arg("--authenticationDatabase").arg(adb);
    }

    if let Some(auth_mechanism) = &config.authentication_mechanism {
        process = process.arg("--authenticationMechanism").arg(auth_mechanism);
    }

    if let Some(collection) = &config.collection {
        process = process.arg("--collection").arg(collection);
    }

    if let Some(query_file) = &config.query_file {
        process = process.arg("--queryFile").arg(query_file.as_os_str());
    }

    process = process
        .arg(connection_string.as_str());

    debug!("Final mongodump command: {}", &process.to_cmdline_lossy());

    process
        .popen()
        .context("Error while starting mongodump process.")
}

fn start_backup(config: &MongoConfig, save_path: &Path, shutdown_rx: &Receiver<()>) -> Result<()> {
    let process = execute_mongodump(config, save_path)?;

    wait_for_subprocess(process, None, shutdown_rx)
}
