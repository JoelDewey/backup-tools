use crate::app_config::AppConfig;
use crate::common::process::wait_for_subprocess;
use crate::pgsql::config;
use crate::pgsql::config::{PgDumpArgs, PostgresConfig};
use anyhow::{anyhow, Context, Result};
use crossbeam::channel::Receiver;
use envy::prefixed;
use std::env;
use std::path::Path;
use subprocess::{Popen, Redirection};
use tracing::{info, trace_span};

pub fn backup_postgres(app_config: &AppConfig, shutdown_rx: &Receiver<()>) -> Result<()> {
    let span = trace_span!("pgsql");
    let _ = span.enter();

    if app_config.postgres_backup_enabled.unwrap_or(false) == false {
        info!("PostgreSQL backup disabled.");
        return Ok(());
    }

    info!("Starting PostgreSQL backup.");
    info!("Loading PostgreSQL configuration variables.");
    let config = get_postgres_config()?;
    let db_name = config
        .database_name
        .as_ref()
        .map(|s| s as &str)
        .unwrap_or_else(|| "db");
    let backup_path = app_config
        .source_path
        .clone()
        .join("db/postgres")
        .join(db_name);
    std::fs::create_dir_all(&backup_path)
        .context("Error while creating path to PostgreSQL backup.")?;

    let args = PgDumpArgs {
        config,
        backup_path,
    };

    start_pg_backup(&args, shutdown_rx)?;

    Ok(())
}

fn get_postgres_config() -> Result<PostgresConfig> {
    env::var(config::POSTGRES_ENV_URL).map_or_else(
        |_| {
            prefixed(config::POSTGRES_PREFIX)
                .from_env()
                .map_err(|e| anyhow!(e))
                .context("Error while mapping PostgresConfig from individual env vars.")
        },
        |s| PostgresConfig::from_url(&s).context("Error while mapping PostgresConfig from URL."),
    )
}

fn execute_pg_dump(config: &PostgresConfig, save_path: &Path) -> Result<Popen> {
    let port = &config.port.unwrap_or(config::DEFAULT_PGSQL_PORT);

    let mut process = subprocess::Exec::cmd("pg_dump")
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Pipe)
        .env("PGPASSWORD", &config.password)
        .arg("-h")
        .arg(&config.host)
        .arg("-p")
        .arg(&port.to_string())
        .arg("-U")
        .arg(&config.username);

    if let Some(db) = &config.database_name {
        process = process.arg("-d").arg(db);
    }

    process
        .arg("-w")
        .arg("--lock-wait-timeout=10")
        .arg("-F")
        .arg("d")
        .arg("-f")
        .arg(save_path.as_os_str())
        .popen()
        .context("Error while starting pg_dump process and returning Popen.")
}

fn start_pg_backup(args: &PgDumpArgs, shutdown_rx: &Receiver<()>) -> Result<()> {
    let process = execute_pg_dump(&args.config, &args.backup_path)?;

    wait_for_subprocess(process, None, shutdown_rx)
}
