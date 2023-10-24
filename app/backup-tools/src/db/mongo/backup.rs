use crate::common::process::wait_for_subprocess;
use crate::db::mongo::config;
use crate::db::mongo::config::MongoConfig;
use anyhow::{anyhow, Context, Result};
use crossbeam::channel::Receiver;
use envy::prefixed;
use std::path::Path;
use subprocess::{Popen, Redirection};
use tracing::{info, trace_span};

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

    let mut process = subprocess::Exec::cmd("mongodump")
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Pipe)
        .arg("--config")
        .arg(config.configuration_file.as_os_str())
        .arg("--host")
        .arg(format!("{}:{}", &config.host, port))
        .arg("--username")
        .arg(&config.username)
        .arg("--gzip")
        .arg("--archive")
        .arg(save_path.join("mongo.gz").as_os_str());

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

    process
        .popen()
        .context("Error while starting mongodump process.")
}

fn start_backup(config: &MongoConfig, save_path: &Path, shutdown_rx: &Receiver<()>) -> Result<()> {
    let process = execute_mongodump(config, save_path)?;

    wait_for_subprocess(process, None, shutdown_rx)
}
