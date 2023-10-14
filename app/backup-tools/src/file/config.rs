use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct RsyncConfig {
    pub timeout: Option<u64>,
    pub exclude_file_path: Option<PathBuf>,
}
