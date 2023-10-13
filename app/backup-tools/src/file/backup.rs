use crate::app_config::AppConfig;
use crate::file::dir_entry_priority::DirEntryPriority;
use crate::file::tar::make_tar_gz;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use crossbeam::channel::Receiver;
use std::collections::BinaryHeap;
use std::fs;
use std::fs::remove_file;
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
        "{}_{}.tar.gz",
        now.format("%F_%H%M%S"),
        &app_config.backup_name
    ));

    let oldest_filepath = get_oldest_backup(&app_config)?;

    info!(filename=%filename.display(), "Creating backup.");
    make_tar_gz(app_config, &filename, shutdown_rx).context("Error while creating tar file.")?;

    if let Some(filepath) = oldest_filepath {
        info!(filepath=%filepath.display(), "Deleting oldest backup as we've reached our max.");
        remove_file(filepath).context("Error while deleting oldest file.")
    } else {
        Ok(())
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

fn get_oldest_backup(app_config: &AppConfig) -> Result<Option<PathBuf>> {
    let dir = &app_config.destination_path;
    if !dir.is_dir() {
        bail!("Went to find the oldest file in the destination directory but was give a path to a file instead.");
    }

    let mut heap: BinaryHeap<DirEntryPriority> = BinaryHeap::new();
    for entry in fs::read_dir(dir)? {
        heap.push(DirEntryPriority::new(entry?)?);
    }

    if heap.len() < app_config.max_number_of_backups as usize {
        Ok(None)
    } else {
        let oldest = heap
            .pop()
            .expect("Expected a PathBuf in the heap but found nothing.");
        Ok(Some(oldest.path))
    }
}
