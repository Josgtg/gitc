use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use crate::{error::CustomResult, Constants, Result};

/// Creates a new git repository, placing it inside `folder_name` if one is provided.
///
/// # Errors
///
/// This function will fail if any of the operations related with the creation of directories and
/// files fail.
pub fn init(folder_name: Option<&OsStr>) -> Result<String> {
    // building root folder
    let repository_path = Constants::repository_path();
    let path = match folder_name {
        Some(name) => PathBuf::from(name).join(repository_path),
        None => repository_path,
    };

    if fs::exists(&path).map_err_with("could not verify folder existance when initializing")? {
        return Ok("The directory is already a git repository".into());
    }

    // creating directory if it didn't exist
    fs::create_dir_all(&path).map_err_with("could not create repository directory when initializing")?;

    // creating subdirectories
    for p in [
        Constants::objects_path(),
        Constants::refs_path(),
        Constants::heads_path(),
    ] {
        fs::create_dir_all(&p).map_err_with(format!("could not create repository subdirectories, specifically: {p:?}"))?;
    }

    // creating default head file
    fs::write(Constants::head_path(), Constants::DEFAULT_HEAD).map_err_with("could not write to HEAD when initializing")?;

    Ok("Created new git repository".into())
}
