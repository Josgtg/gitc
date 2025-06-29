use std::path::Path;
use std::fs::{self, File};

use crate::Constants;
use crate::Result;

/// Returns true if the path returned by `Constants::repository_folder_path` exists.
///
/// # Errors
///
/// This function will panic if `std::fs::exists` returns an Err.
pub fn repository_exists() -> bool {
    fs::exists(Constants::repository_folder_path()).unwrap()
}

/// Creates a folder through the `std::fs::create_dir_all` function.
pub fn create_folder(path: &Path) -> Result<()> {
    Ok(fs::create_dir_all(path)?)
}

/// Creates a file through the `std::fs::File::create` function, returning the created file.
pub fn create_file(path: &Path) -> Result<File> {
    Ok(fs::File::create(path)?)
}

/// Reads a file trough the `std::fs::read` function, returning the bytes of the read file.
pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    Ok(fs::read(path)?)
}
