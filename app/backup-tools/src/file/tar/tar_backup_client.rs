use crate::app_config::AppConfig;
use crate::common::process::{create_command, wait_for_child};
use crate::file::backup_client::BackupClient;
use crate::file::tar::config::TarConfig;
use anyhow::Context;
use anyhow::Result;
use crossbeam::channel::Receiver;
use std::path::Path;
use std::process::Child;
use std::time::Duration;
use tracing::trace_span;

pub const COMPRESSED_CONFIG_PREFIX: &str = "COMPRESSED_";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60 * 60; // 60 minutes

pub struct TarBackupClient<'a> {
    app_config: &'a AppConfig,
    tar_config: TarConfig,
}

impl<'a> TarBackupClient<'a> {
    pub fn new(app_config: &'a AppConfig) -> Result<TarBackupClient<'a>> {
        let tar_config = envy::prefixed(COMPRESSED_CONFIG_PREFIX)
            .from_env::<TarConfig>()
            .context("Error while loading tar config.")?;

        Ok(TarBackupClient {
            app_config,
            tar_config,
        })
    }
    fn execute_tar(&self, destination_filepath: &Path) -> Result<Child> {
        let mut builder = create_command("tar");
        let mut builder_ref = &mut builder;

        builder_ref
            .arg("-zcvf")
            .arg(destination_filepath.as_os_str());

        if let Some(excludes) = &self.tar_config.exclude_file_path {
            builder_ref = builder_ref.arg("--exclude-from").arg(excludes.as_os_str());
        }

        builder_ref
            .arg("-C")
            .arg(self.app_config.source_path.as_os_str())
            .arg(".")
            .spawn()
            .context("Error while starting tar process and returning Popen.")
    }
}

impl<'a> BackupClient for TarBackupClient<'a> {
    fn run_backup(&self, filename: &Path, shutdown_rx: &Receiver<()>) -> Result<()> {
        let mut destination_filepath = self.app_config.destination_path.clone().join(filename);
        destination_filepath.set_extension("tar.gz");
        let timeout = self.tar_config.timeout.map_or_else(
            || Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            Duration::from_secs,
        );

        let span = trace_span!("tar");
        let _ = span.enter();
        let process = self.execute_tar(&destination_filepath)?;
        wait_for_child(process, Some(timeout), shutdown_rx)
    }
}
