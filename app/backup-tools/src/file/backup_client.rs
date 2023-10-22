use anyhow::Result;
use crossbeam::channel::Receiver;
use std::path::Path;

pub trait BackupClient {
    fn run_backup(&self, base_filename: &Path, shutdown_rx: &Receiver<()>) -> Result<()>;
}
