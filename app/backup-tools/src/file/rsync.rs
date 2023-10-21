use crate::app_config::AppConfig;
use crate::common::process::wait_for_subprocess;
use crate::file::config::RsyncConfig;
use anyhow::{Context, Result};
use crossbeam::channel::Receiver;
use std::path::{Path, PathBuf};
use std::time::Duration;
use subprocess::{Popen, Redirection};

pub const INCREMENTAL_CONFIG_PREFIX: &str = "INCR_";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60 * 5; // 5 minutes

pub fn make_incremental_backup(
    app_config: &AppConfig,
    name: &Path,
    previous_backup: Option<&PathBuf>,
    shutdown_rx: &Receiver<()>,
) -> Result<()> {
    let destination_filepath = app_config.destination_path.clone().join(name);
    let rsync_config = envy::prefixed(INCREMENTAL_CONFIG_PREFIX)
        .from_env::<RsyncConfig>()
        .context("Error while loading tar config.")?;
    let timeout = rsync_config.timeout.map_or_else(
        || Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        |v| Duration::from_secs(v),
    );

    let process = execute_rsync(
        &rsync_config,
        &app_config.source_path,
        &destination_filepath,
        previous_backup,
    )?;
    wait_for_subprocess(process, Some(timeout), shutdown_rx)
}

fn execute_rsync(
    config: &RsyncConfig,
    source_path: &Path,
    destination_filepath: &Path,
    previous_backup: Option<&PathBuf>,
) -> Result<Popen> {
    let mut args = String::from("-azP");
    let destination_owner = &config.destination_owner;
    let destination_group = &config.destination_group;

    if destination_owner.is_some() {
        args.push('o');
    }

    if destination_group.is_some() {
        args.push('g');
    }

    let mut builder = subprocess::Exec::cmd("rsync")
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Pipe)
        .arg(args)
        .arg("--delete");

    if destination_owner.is_some() || destination_group.is_some() {
        let owner = destination_owner
            .as_ref()
            .map(|s| s as &str)
            .unwrap_or_else(|| "");
        let group = destination_group
            .as_ref()
            .map(|s| s as &str)
            .unwrap_or_else(|| "");
        builder = builder
            .arg("--chown")
            .arg(format!("{}:{}", owner, group))
            .arg("--super");
    }

    if let Some(whole_file) = &config.whole_file {
        if *whole_file {
            builder = builder.arg("--whole-file");
        }
    }

    if let Some(excludes) = &config.exclude_file_path {
        builder = builder.arg("--exclude-from").arg(excludes.as_os_str());
    }

    if let Some(previous) = previous_backup {
        builder = builder.arg("--link-dest").arg(previous.as_os_str());
    }

    // Adds an extra slash at the end of the source path, which indicates to rsync that
    // it should copy the contents of the directory but not the directory itself.
    let mut final_source = PathBuf::from(source_path);
    final_source.push("");

    builder
        .arg(final_source.as_os_str())
        .arg(destination_filepath.as_os_str())
        .popen()
        .context("Error while starting tar process and returning Popen.")
}
