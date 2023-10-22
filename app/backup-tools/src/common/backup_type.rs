use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackupType {
    Incremental,
    Compressed,
}
