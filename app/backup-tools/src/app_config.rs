use crate::common::BackupType;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Default)]
pub struct AppConfig {
    pub backup_name: String,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub max_number_of_backups: u64,
    pub scale_deployment_enabled: Option<bool>,
    pub postgres_backup_enabled: Option<bool>,
    pub mongo_backup_enabled: Option<bool>,
    pub backup_type: Option<BackupType>,
}
