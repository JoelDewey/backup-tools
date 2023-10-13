use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::path::PathBuf;
use url;

pub const POSTGRES_PREFIX: &str = "POSTGRES_";
pub const POSTGRES_ENV_URL: &str = "POSTGRES_URL";
pub const DEFAULT_PGSQL_PORT: u16 = 5432;

#[derive(Debug, Deserialize)]
pub struct PostgresConfig {
    pub host: String,
    pub port: Option<u16>,
    pub database_name: String,
    pub username: String,
    pub password: String,
}

impl PostgresConfig {
    pub fn from_url(url_str: &str) -> Result<PostgresConfig> {
        let url = url::Url::parse(url_str)?;
        let database_name = url
            .path_segments()
            .ok_or(anyhow!(
                "Could not retrieve path segments for PGSQL database name. defined."
            ))?
            .next()
            .ok_or(anyhow!("No PGSQL database name."))?;

        Ok(PostgresConfig {
            host: url
                .host()
                .ok_or(anyhow!("No PGSQL host provided."))?
                .to_string(),
            port: url.port(),
            database_name: String::from(database_name),
            username: String::from(url.username()),
            password: String::from(url.password().unwrap_or("")),
        })
    }
}

pub struct PgDumpArgs {
    pub config: PostgresConfig,
    pub backup_path: PathBuf,
}
