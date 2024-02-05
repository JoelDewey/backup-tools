use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::path::PathBuf;

pub const POSTGRES_PREFIX: &str = "POSTGRES_";
pub const POSTGRES_ENV_URL: &str = "POSTGRES_URL";
pub const DEFAULT_PGSQL_PORT: u16 = 5432;

#[derive(Debug, Deserialize)]
pub struct PostgresConfig {
    pub host: String,
    pub port: Option<u16>,
    pub database_name: Option<String>,
    pub username: String,
    pub password: String,
}

impl PostgresConfig {
    pub fn from_url(url_str: &str) -> Result<PostgresConfig> {
        let url = url::Url::parse(url_str)?;
        let mut database_name: Option<String> = None;

        if let Some(mut segments) = url.path_segments() {
            database_name = segments.next().and_then(|db| Option::from(db.to_string()));
        }

        Ok(PostgresConfig {
            host: url
                .host()
                .ok_or(anyhow!("No PGSQL host provided."))?
                .to_string(),
            port: url.port(),
            database_name,
            username: String::from(url.username()),
            password: String::from(url.password().unwrap_or("")),
        })
    }
}

pub struct PgDumpArgs {
    pub config: PostgresConfig,
    pub backup_path: PathBuf,
}

#[cfg(test)]
mod tests {
    use crate::db::pgsql::config::PostgresConfig;

    #[test]
    pub fn from_url_given_url_creates() {
        // Arrange
        let host = "localhost";
        let port = 5432;
        let user = "test";
        let pass = "pass";
        let db_name = "db";
        let url = format!("postgres://{}:{}@{}:{}/{}", user, pass, host, port, db_name);

        // Act
        let actual_result = PostgresConfig::from_url(&url);

        // Assert
        assert!(actual_result.is_ok());
        let actual = actual_result.unwrap();
        assert_eq!(user, actual.username);
        assert_eq!(pass, actual.password);
        assert_eq!(host, actual.host);

        // Port should be set since we explicitly defined it.
        assert!(actual.port.is_some());
        assert_eq!(port, actual.port.unwrap());

        assert!(actual.database_name.is_some());
        assert_eq!(db_name, actual.database_name.unwrap());
    }
}
