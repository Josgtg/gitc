use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;

use crate::Constants;

/// Reads the path stored inside the HEAD file.
///
/// # Errors
///
/// This function will fail if the HEAD file could not be opened or read from.
pub fn get_current_branch_path() -> Result<PathBuf> {
    let bytes = fs::read(Constants::head_path()).context("could not read from HEAD file")?;
    Ok(PathBuf::from(OsString::from_vec(bytes)))
}

/// Returns `path` relative to `base`.
///
/// # Errors
///
/// This function will return `None` if `base` was not a prefix of `path`.
pub fn relative_path(path: &Path, base: &Path) -> Option<PathBuf> {
    path.strip_prefix(base).map(PathBuf::from).ok()
}

/// Returns the path divided by forward slashes.
pub fn format_path(path: &Path) -> OsString {
    let mut formatted = OsString::new();
    let mut prev: &OsStr = OsStr::new("");
    for (i, p) in path.iter().enumerate() {
        if i != 0 && prev != "/" {
            // doing this to avoid placing a forward slash at the end or when the path before is a
            // forward slash
            formatted.push("/");
        }
        formatted.push(p);
        prev = p;
    }
    formatted
}

/// Returns the path without useless characters.
///
/// If the `absolute` flag is set, it will not strip the forward slash from the path.
pub fn clean_path(path: PathBuf, absolute: bool) -> PathBuf {
    let cleaned: PathBuf = if path.starts_with("./") {
        path.strip_prefix("./").unwrap().into()
    } else if path.starts_with("/") && !absolute {
        path.strip_prefix("/").unwrap().into()
    } else {
        path
    };

    cleaned
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

// Returns the files in `path` that are not inside a .gitignore file in the same directory.
//
// # Errors
//
// This function can fail if it couldn't get the files inside `path` or could not filter from the
// gitignore.
pub fn read_not_ignored_paths(path: &Path) -> Result<Vec<PathBuf>> {
    let all_paths = read_dir_paths(&path).context("could not read root directory entries")?;
    Ok(crate::gitignore::not_in_gitignore(&path, all_paths)?)
}

// Tests

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use crate::fs::path::{format_path, relative_path};

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
