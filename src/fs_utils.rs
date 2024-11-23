use std::fs;
use std::path::PathBuf;
use rand::seq::SliceRandom;
use rand::thread_rng;

#[allow(unused)]
pub fn get_random_file_from_directory(dir_path: &str) -> Option<PathBuf> {
    // Read the directory entries
    let entries = fs::read_dir(dir_path).ok()?;

    // Collect all file paths
    let files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())  // Filter out any errors
        .filter_map(|entry| {
            let path = entry.path();
            // Only include files (skip directories and other non-file entries)
            if path.is_file() {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    // Randomly choose one file if the list is not empty
    files.choose(&mut thread_rng()).cloned()
}