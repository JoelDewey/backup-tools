use crate::app_config::AppConfig;
use crate::common::BackupType;
use crate::file::backup_client::BackupClient;
use crate::file::dir_entry_priority::DirEntryPriority;
use crate::file::{rsync, tar};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use crossbeam::channel::Receiver;
use std::collections::BinaryHeap;
use std::fs;
use std::fs::remove_dir_all;
use std::path::{Path, PathBuf};
use tracing::{debug, enabled, info, Level, warn};

pub fn backup_files(app_config: &AppConfig, shutdown_rx: &Receiver<()>) -> Result<()> {
    info!("Beginning file backup.");

    let has_nonempty_files = has_nonempty_files(&app_config.source_path)?;
    if !has_nonempty_files {
        warn!("Source path is empty, consists of only empty directories, and/or consists only of zero byte files. No backup will be made.");
        return Ok(());
    }

    let now: DateTime<Utc> = Utc::now();
    let filename = PathBuf::from(format!(
        "{}_{}",
        now.format("%F_%H%M%S"),
        &app_config.backup_name
    ));

    let previous_backups = get_previous_backups(app_config)?;

    let latest = previous_backups.peek().map(|e| e.path.as_path());
    let client =
        get_backup_client(app_config, latest).context("Failed to create backup client.")?;

    info!(filename=%filename.display(), "Creating backup.");
    client
        .run_backup(&filename, shutdown_rx)
        .context("Error while making backup.")?;

    let backup_count = previous_backups.len() + 1; // Includes the backup we just made.
    if app_config.max_number_of_backups == 0
        || backup_count < app_config.max_number_of_backups as usize
    {
        Ok(())
    } else {
        info!(count=&backup_count, "Deleting the oldest backups as we've reached our max.");
        let skip = app_config.max_number_of_backups.saturating_sub(1);
        let mut count = 0;
        
        if enabled!(Level::DEBUG) { 
            previous_backups
                .iter()
                .for_each(|b| debug!(path=%&b.path.display(), modified=?&b.created, "Found previous backup path."))
        }
        
        previous_backups
            .iter()
            .skip(skip as usize)
            .try_for_each(|b| {
                remove_dir_all(&b.path)
                    .context("Error while deleting older backup.")
                    .map(|r| {
                        count += 1;
                        info!(path=%&b.path.display(), "Deleted backup at the given path.");
                        r
                    })
            })
            .map(|r| {
                info!(total_deletes = count, "Finished deleting oldest backups.");
                r
            })
    }
}

fn has_nonempty_files(dir: &Path) -> Result<bool> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let result = has_nonempty_files(&path)?;
                if result {
                    return Ok(true);
                }
            } else if fs::metadata(path)?.len() > 0 {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn get_previous_backups(app_config: &AppConfig) -> Result<BinaryHeap<DirEntryPriority>> {
    let dir = &app_config.destination_path;
    if !dir.is_dir() {
        bail!("Went to find the oldest file in the destination directory but was give a path to a file instead.");
    }

    let mut heap: BinaryHeap<DirEntryPriority> = BinaryHeap::new();
    for entry in fs::read_dir(dir)? {
        heap.push(DirEntryPriority::new(entry?)?);
    }

    Ok(heap)
}

fn get_backup_client<'a>(
    app_config: &'a AppConfig,
    previous_backup: Option<&Path>,
) -> Result<Box<dyn BackupClient + 'a>> {
    let result: Box<dyn BackupClient + 'a> = match app_config
        .backup_type
        .as_ref()
        .unwrap_or(&BackupType::Incremental)
    {
        BackupType::Compressed => Box::new(tar::TarBackupClient::new(app_config)?),
        BackupType::Incremental => {
            Box::new(rsync::RsyncBackupClient::new(app_config, previous_backup)?)
        }
    };

    Ok(result)
}
