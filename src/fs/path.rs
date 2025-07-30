use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
use anyhow::Result;

use crate::hashing::Hash;
use crate::Constants;

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

/// Returns all the paths of the files and subdirectories inside of `dir`.
///
/// # Errors
///
/// This function will fail if:
/// `dir` did not exist.
/// `dir` was not a directory.
/// Could not get the files inside of `dir`.
pub fn read_dir_paths(path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let entries = std::fs::read_dir(path).context("could not get directory entries")?;
    for e in entries {
        paths.push(e?.path());
    }
    Ok(paths)
}

/// Returns the files in `path` that are not inside a .gitignore file in the same directory.
///
/// # Errors
///
/// This function can fail if it couldn't get the files inside `path` or could not filter from the
/// gitignore.
pub fn read_not_ignored_paths(path: &Path) -> Result<Vec<PathBuf>> {
    let all_paths = read_dir_paths(path).context("could not read root directory entries")?;
    crate::gitignore::not_in_gitignore(path, all_paths)
}

// Tests

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use crate::utils::path::{format_path, relative_path};

    #[test]
    pub fn relative_path_test() {
        let path = PathBuf::from(".git/index");
        let base = env::current_dir().expect("failed to get current dir");
        let joined = base.join(&path);

        assert_eq!(
            path,
            relative_path(&joined, &base).expect("failed to get relative path")
        );

        let base2 = PathBuf::from("/home/juano/");

        assert!(relative_path(&joined, &base2).is_none());

        assert!(relative_path(&PathBuf::new(), &PathBuf::new()).is_some())
    }

    #[test]
    pub fn format_path_test() {
        let mut path = PathBuf::new();
        path.push("/");
        path.push("home");
        path.push("josgtg");
        path.push("games");
        path.push("game.exe");
        let objective = "/home/josgtg/games/game.exe";

        assert_eq!(objective, format_path(&path))
    }
}
