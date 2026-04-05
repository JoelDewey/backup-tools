use anyhow::Result;
use std::cmp::Ordering;
use std::fs::DirEntry;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Eq)]
pub struct DirEntryPriority {
    pub path: PathBuf,
    pub(crate) created: SystemTime,
}

impl DirEntryPriority {
    pub fn new(entry: DirEntry) -> Result<DirEntryPriority> {
        Ok(DirEntryPriority {
            path: entry.path(),
            created: entry.metadata()?.modified()?,
        })
    }
}

impl PartialEq for DirEntryPriority {
    fn eq(&self, other: &Self) -> bool {
        self.created == other.created
    }
}

impl PartialOrd for DirEntryPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DirEntryPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.created.cmp(&other.created)
    }
}

#[cfg(test)]
mod test {
    use crate::file::dir_entry_priority::DirEntryPriority;
    use std::cmp::Ordering;
    use std::env::temp_dir;
    use std::fs;
    use std::fs::File;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn cmp_givenolder_returnsless() {
        // Arrange
        let temp_dir = temp_dir().join("cmp_givenolder_returnsgreater");
        fs::create_dir(&temp_dir).expect("Failed to create temporary file.");

        let older_path = temp_dir.join("00_cmp_givenolder_returnsgreater_older.txt");
        File::create(older_path).expect("Failed to create older file.");
        sleep(Duration::from_secs(1));

        let newer_path = temp_dir.join("01_cmp_givenolder_returnsgreater_newer.txt");
        File::create(newer_path).expect("Failed to create newer file.");

        let mut files = fs::read_dir(&temp_dir)
            .expect("Failed to read files out of temporary directory.")
            .map(|r| {
                DirEntryPriority::new(r.unwrap()).expect("Failed to construct DirEntryPriority")
            })
            .collect::<Vec<DirEntryPriority>>();
        files.sort_by(|first, second| first.path.cmp(&second.path));
        fs::remove_dir_all(&temp_dir).expect("Failed to delete files.");

        // Act + Assert
        assert_eq!(files[0].cmp(&files[1]), Ordering::Less);
    }

    #[test]
    fn cmp_given_newer_returns_greater() {
        let temp_dir = temp_dir().join("cmp_given_newer_returns_greater");
        fs::create_dir(&temp_dir).expect("Failed to create temporary directory.");

        let older_path = temp_dir.join("00_older.txt");
        File::create(older_path).expect("Failed to create older file.");
        sleep(Duration::from_secs(1));

        let newer_path = temp_dir.join("01_newer.txt");
        File::create(newer_path).expect("Failed to create newer file.");

        let mut files = fs::read_dir(&temp_dir)
            .expect("Failed to read temporary directory.")
            .map(|r| DirEntryPriority::new(r.unwrap()).expect("Failed to construct DirEntryPriority"))
            .collect::<Vec<DirEntryPriority>>();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        fs::remove_dir_all(&temp_dir).expect("Failed to delete temporary directory.");

        assert_eq!(files[1].cmp(&files[0]), Ordering::Greater);
    }

    #[test]
    fn binary_heap_pops_newest_entry_first() {
        let temp_dir = temp_dir().join("binary_heap_pops_newest_first");
        fs::create_dir(&temp_dir).expect("Failed to create temporary directory.");

        let older_path = temp_dir.join("00_heap_older.txt");
        File::create(older_path).expect("Failed to create older file.");
        sleep(Duration::from_secs(1));

        let newer_path = temp_dir.join("01_heap_newer.txt");
        File::create(newer_path).expect("Failed to create newer file.");

        let mut heap: std::collections::BinaryHeap<DirEntryPriority> =
            std::collections::BinaryHeap::new();
        fs::read_dir(&temp_dir)
            .expect("Failed to read temporary directory.")
            .for_each(|entry| heap.push(DirEntryPriority::new(entry.unwrap()).unwrap()));
        fs::remove_dir_all(&temp_dir).expect("Failed to delete temporary directory.");

        let first = heap.pop().unwrap();
        assert!(
            first.path.ends_with("01_heap_newer.txt"),
            "Expected the newest file to be popped first, got: {:?}",
            first.path
        );
    }
}
