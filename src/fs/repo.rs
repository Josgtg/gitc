use anyhow::{Context, Result};

use super::get_current_branch_path;

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
