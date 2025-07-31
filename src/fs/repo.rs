use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Context, Result};

use crate::Constants;
use crate::hashing::Hash;

/// Reads the path stored inside the HEAD file.
//
/// # Errors
///
/// This function will fail if the HEAD file could not be opened or read from.
pub fn get_current_branch_path() -> Result<PathBuf> {
    let bytes = std::fs::read(Constants::head_path()).context("could not read from HEAD file")?;
    let path_str = String::from_utf8_lossy(&bytes);

    let stripped_path_str = path_str
        .trim_end()  // Important to remove ending newlines
        .strip_prefix(Constants::HEAD_CONTENT_HEADER)
        .context("HEAD file had an incorrect header")?;

    let relative_path = PathBuf::from(stripped_path_str);

    Ok(Constants::repository_path().join(relative_path))
}

/// Returns the hash of the last commit on the current branch. More specifically, the hash inside
/// the file HEAD points to.
///
/// # Returns
///
/// This function returns a Result of an option of a Hash. The result might be `Err` if it was not
/// possible to read from the file or get the path HEAD pointed to, while the Option inside might be
/// `None` if there were no commits yet.
pub fn get_last_commit_hash() -> Result<Option<Hash>> {
    let path = get_current_branch_path().context("could not get current branch path")?;

    if !path.exists() {
        return Ok(None);
    }

    let bytes = std::fs::read(path).context("could not read current branch")?;
    let str = std::str::from_utf8(&bytes).context("could not read current branch as a string")?;

    let hash = Hash::from_str(str.trim()).context("could not create a hash from the data read")?;

    Ok(Some(hash))
}

/// Returns the name of the branch HEAD points to.
///
/// # Errors
///
/// This function will fail if it could not read from the HEAD file.
pub fn get_current_branch_name() -> Result<String> {
    let path_head_points_to = get_current_branch_path().context("could not read branch path")?;

    Ok(path_head_points_to
        .components()
        .last()
        .context("path in HEAD was empty?")?
        .as_os_str()
        .to_string_lossy()
        .to_string())
}
