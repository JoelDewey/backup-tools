use crate::app_config::AppConfig;
use crate::common::process::wait_for_subprocess;
use crate::file::config::TarConfig;
use anyhow::{Context, Result};
use crossbeam::channel::Receiver;
use std::path::Path;
use std::time::Duration;
use subprocess::{Popen, Redirection};

pub const TAR_CONFIG_PREFIX: &str = "TAR_";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60 * 5; // 5 minutes

pub fn make_tar_gz(
    app_config: &AppConfig,
    filename: &Path,
    shutdown_rx: &Receiver<()>,
) -> Result<()> {
    let destination_filepath = app_config.destination_path.clone().join(filename);
    let tar_config = envy::prefixed(TAR_CONFIG_PREFIX)
        .from_env::<TarConfig>()
        .context("Error while loading tar config.")?;
    let timeout = tar_config.timeout.map_or_else(
        || Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        |v| Duration::from_secs(v),
    );

    let process = execute_tar(&tar_config, &app_config.source_path, &destination_filepath)?;
    wait_for_subprocess(process, Some(timeout), shutdown_rx)
}

fn execute_tar(
    config: &TarConfig,
    source_path: &Path,
    destination_filepath: &Path,
) -> Result<Popen> {
    let mut builder = subprocess::Exec::cmd("tar")
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Pipe)
        .arg("-zcvf")
        .arg(destination_filepath.as_os_str());

    if let Some(excludes) = &config.exclude_file_path {
        builder = builder.arg("--exclude-from").arg(excludes.as_os_str());
    }

    builder
        .arg("-C")
        .arg(source_path.as_os_str())
        .arg(".")
        .popen()
        .context("Error while starting tar process and returning Popen.")
}
