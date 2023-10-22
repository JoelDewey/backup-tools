use std::path::Path;
use crossbeam::channel::Receiver;
use anyhow::Result;

pub trait BackupClient {
    fn run_backup(&self, filename: &Path, shutdown_rx: &Receiver<()>) -> Result<()>;
}