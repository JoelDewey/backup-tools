use crate::app_config::AppConfig;
use crate::file::dir_entry_priority::DirEntryPriority;
use crate::file::rsync::make_incremental_backup;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use crossbeam::channel::Receiver;
use std::collections::BinaryHeap;
use std::fs;
use std::fs::remove_dir_all;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

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

    let latest = previous_backups.peek().map(|e| &e.path);
    info!(filename=%filename.display(), "Creating backup.");
    make_incremental_backup(app_config, &filename, latest, shutdown_rx)
        .context("Error while making incremental backup.")?;

    let backup_count = previous_backups.len() + 1; // Includes the backup we just made.
    if app_config.max_number_of_backups == 0
        || backup_count < app_config.max_number_of_backups as usize
    {
        Ok(())
    } else {
        info!("Deleting the oldest backups as we've reached our max.");
        let skip = app_config.max_number_of_backups.checked_sub(1).unwrap_or(0);
        let mut count = 0;
        previous_backups
            .iter()
            .skip(skip as usize)
            .try_for_each(|b| {
                remove_dir_all(&b.path)
                    .context("Error while deleting older backup.")
                    .and_then(|r| {
                        count += 1;
                        info!(path=%&b.path.display(), "Deleted backup at the given path.");
                        Ok(r)
                    })
            })
            .and_then(|r| {
                info!(total_deletes=count, "Finished deleting oldest backups.");
                Ok(r)
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
            } else {
                if fs::metadata(path)?.len() > 0 {
                    return Ok(true);
                }
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
