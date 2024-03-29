use crate::common::process::wait_for_child_with_redirection;
use crate::db::mongo::config;
use crate::db::mongo::config::MongoConfig;
use anyhow::{anyhow, Context, Result};
use crossbeam::channel::Receiver;
use envy::prefixed;
use std::fs::create_dir_all;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use tracing::{debug, info, trace_span};
use url::Url;

pub fn backup_mongo(base_backup_path: &Path, shutdown_rx: &Receiver<()>) -> Result<()> {
    let span = trace_span!("mongo");
    let _entered = span.enter();

    info!("Starting MongoDB backup.");
    let config = get_mongo_config()?;
    let db_name = config
        .database_name
        .as_ref()
        .map(|s| s as &str)
        .unwrap_or_else(|| "mongodb");
    let backup_path = base_backup_path.join(format!("mongo/{}", db_name));
    create_dir_all(&backup_path)
        .context("Failed to create backup directory during MongoDB backup.")?;

    start_backup(&config, &backup_path, shutdown_rx)?;

    Ok(())
}

fn get_mongo_config() -> Result<MongoConfig> {
    prefixed(config::MONGO_PREFIX)
        .from_env()
        .map_err(|e| anyhow!(e))
        .context("Error while mapping MongoConfig from individual env vars.")
}

fn execute_mongodump(config: &MongoConfig, save_path: &Path) -> Result<Child> {
    let port = &config.port.unwrap_or(config::DEFAULT_PORT);
    let connection_string = Url::parse(&format!("mongodb://{}:{}", &config.host, port))
        .context("Error encountered while creating MongoDB connection string.")?;

    // mongodump writes its normal output to stderr as it can be configured to output the backup data to stdout.
    // In backup-tools, mongodump is configured to write out backup data to the mongo.gz archive.
    // Therefore, stderr will be merged to stdout so that the correct logging severity is chosen.
    // Actual errors are reported via the status code per MongoDB's developers.
    // https://jira.mongodb.org/browse/TOOLS-1484

    let mut process = Command::new("mongodump");
    let mut process_ref = &mut process;

    process_ref
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .current_dir(save_path)
        .arg("--config")
        .arg(config.configuration_file.as_os_str())
        .args(["--username", &config.username])
        .arg("--gzip")
        .arg("--archive=mongo.gz");

    if let Some(db) = &config.database_name {
        process_ref = process_ref.args(["--db", db]).arg("--dumpDbUsersAndRoles");
    }

    if let Some(adb) = &config.authentication_database_name {
        process_ref = process_ref.args(["--authenticationDatabase", adb]);
    }

    if let Some(auth_mechanism) = &config.authentication_mechanism {
        process_ref = process_ref.args(["--authenticationMechanism", auth_mechanism]);
    }

    if let Some(collection) = &config.collection {
        process_ref = process_ref.args(["--collection", collection]);
    }

    if let Some(query_file) = &config.query_file {
        process_ref = process_ref.arg("--queryFile").arg(query_file.as_os_str());
    }

    process_ref = process_ref.arg(connection_string.as_str());

    debug!("Final mongodump command: {:?}", &process_ref);

    process_ref
        .spawn()
        .context("Error while starting mongodump process.")
}

fn start_backup(config: &MongoConfig, save_path: &Path, shutdown_rx: &Receiver<()>) -> Result<()> {
    let process = execute_mongodump(config, save_path)?;

    wait_for_child_with_redirection(process, None, shutdown_rx, true)
}
