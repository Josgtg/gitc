use std::ffi::OsString;
use std::path::PathBuf;

use crate::fs;
use crate::{Constants, Result};

const PATTERN_EVERY_FILE: &'static str = ".";

pub fn add(files: &Vec<OsString>) -> Result<String> {
    if files.is_empty() {
        return Ok("There were no files to add".into());
    }

    let folder_path = Constants::repository_folder_path();

    let files_to_ignore = fs::read_gitignore(&folder_path)?;

    let paths: Vec<PathBuf>;
    if files.first().expect("file did not have first element despite being checked for emptiness") == PATTERN_EVERY_FILE {
        // pattern to add every file
        paths = fs::read_dir_paths(&folder_path)?;
    } else {
        paths = files.iter().map(|p| PathBuf::from(p)).collect();
    }

    // Discarding ignored files
    let filtered_paths: Vec<PathBuf> = paths
        .into_iter()
        .map(|p| {
            fs::relative_path(&p, &folder_path).unwrap_or(p)
        })
        .filter(|p| !files_to_ignore.contains(p))
        .collect();

    for p in filtered_paths {
        add_dir(p).expect("failed to add dir");
    }

    Ok("Added files successfully".into())
}

/// This function calls itself recursively for every subdirectory inside of `dir`, until there are
/// no more subdirectories, calling `add_file` for every file inside `dir`.
fn add_dir(dir: PathBuf) -> Result<()> {
    for p in fs::read_dir_paths(&dir)? {
        if p.is_dir() {
            add_dir(p)?
        } else {
            add_file(p)?
        }
    }
    Ok(())
}

// TODO
fn add_file(file: PathBuf) -> Result<()> {
    todo!()
}
