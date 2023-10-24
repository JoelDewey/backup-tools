use serde::Deserialize;
use std::path::PathBuf;

pub const MONGO_PREFIX: &str = "MONGO_";
pub const DEFAULT_PORT: u16 = 27017;

#[derive(Deserialize, Debug)]
pub struct MongoConfig {
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub configuration_file: PathBuf,
    pub database_name: Option<String>,
    pub authentication_database_name: Option<String>,
    pub authentication_mechanism: Option<String>,
    pub collection: Option<String>,
    pub query_file: Option<PathBuf>,
}
