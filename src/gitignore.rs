use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::Constants;

use crate::utils::path::{clean_path, relative_path};

/// Reads a .gitignore file inside of `path`, returning a HashSet including all the files listed (read by line).
///
/// This function will skip files in the gitignore that have text that could not be interpreted as a `String`.
///
/// This function expects the .gitignore file to be in the root of the given path.
///
/// # Errors
///
/// This function will fail if the .gitignore file could not been opened.
pub fn read_gitignore(path: &Path) -> Result<HashSet<PathBuf>> {
    let mut set: HashSet<PathBuf> = HashSet::new();
    // always adding repository path as a path to ignore no matter what
    set.insert(PathBuf::from(Constants::REPOSITORY_FOLDER_NAME));

    let gitignore_path = path.join(Constants::GITIGNORE_FILE_NAME);
    if !std::fs::exists(&gitignore_path).context("could not check gitignore file existance")? {
        return Ok(set);
    }

    let gitignore = File::open(gitignore_path).context("could not open gitignore file")?;

    let reader = BufReader::new(gitignore);
    let mut path: PathBuf;
    for line in reader.lines().map_while(Result::ok) {
        path = PathBuf::from(line);
        set.insert(clean_path(path, false));
    }

    Ok(set)
}

/// Returns a list of files not in the .gitignore file (filters `paths_to_filter`).
///
/// This function looks for a .gitignore file directly inside of `path`.
///
/// It also checks every path in `paths_to_filter` as if it was relative to `path`.
///
/// # Errors
///
/// This function can fail if the .gitignore file could not be read.
pub fn not_in_gitignore(
    path_to_look: &Path,
    paths_to_filter: Vec<PathBuf>,
) -> Result<Vec<PathBuf>> {
    // Always add .gitignore despite it being hidden
    let always_add: HashSet<PathBuf> =
        HashSet::from([PathBuf::from(Constants::GITIGNORE_FILE_NAME)]);

    let files_to_ignore = read_gitignore(path_to_look).context("could not read .gitignore file")?;

    let relative_paths: Vec<PathBuf> = paths_to_filter
        .into_iter()
        .map(|p| relative_path(&p, path_to_look).unwrap_or(p))
        .collect();

    Ok(relative_paths
        .into_iter()
        .filter(|p|
            // ignoring files in .gitignore or hidden paths only including special files
            always_add.contains(p) || (!files_to_ignore.contains(p) && !p.starts_with(".")))
        .collect())
}
