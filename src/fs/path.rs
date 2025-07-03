use std::collections::HashSet;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;

use crate::error::CustomResult;
use crate::Constants;
use crate::Result;

/// Reads a .gitignore file inside of `path`, returning a HashSet including all the files listed (read by line).
///
/// This function will skip files in the gitignore that have text that could not be interpreted as a `String`.
///
/// # Errors
///
/// This function will fail if the .gitignore file could not been opened.
pub fn read_gitignore(path: &Path) -> Result<HashSet<PathBuf>> {
    let mut set: HashSet<PathBuf> = HashSet::new();
    // always adding repository path as a path to ignore no matter what
    set.insert(PathBuf::from(Constants::REPOSITORY_FOLDER_NAME));

    let gitignore_path = path.join(Constants::GITIGNORE_FILE_NAME);
    if !std::fs::exists(&gitignore_path).map_err_with("could not check gitignore file existance")? {
        return Ok(set);
    }

    let gitignore = File::open(gitignore_path).map_err_with("could not open gitignore file")?;

    let reader = BufReader::new(gitignore);
    for line in reader.lines().map_while(Result::ok) {
        set.insert(PathBuf::from(line));
    }

    Ok(set)
}

/// Returns a list of files not in the .gitignore file.
///
/// This function looks for a .gitignore file in the path returned by
/// Constants::repository_folder_path.
///
/// # Errors
///
/// This function can fail if the .gitignore file could not be read.
pub fn not_in_gitignore(files: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let root_path = Constants::repository_folder_path();
    let files_to_ignore =
        read_gitignore(&root_path).map_err_with("could not read .gitignore file")?;
    Ok(files
        .into_iter()
        .map(|p| relative_path(&p, &root_path).unwrap_or(p))
        .filter(|p| !files_to_ignore.contains(p))
        .collect())
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
    let entries = std::fs::read_dir(path).map_err_with("could not get directory entries")?;
    for e in entries {
        paths.push(e?.path());
    }
    Ok(paths)
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
