use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn read_input(input_path: &Path) -> Result<Vec<PathBuf>> {
    let input_files: Vec<PathBuf> = if input_path.is_dir() {
        std::fs::read_dir(input_path)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect()
    } else {
        vec![input_path.to_path_buf()]
    };
    Ok(input_files)
}
