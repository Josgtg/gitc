use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::error::WarnUnwrap;
use crate::gitignore;
use crate::index::IndexEntryCache;

/// Struct that represents a file which content is buffered.
///
/// It's use is to not load bytes into memory when reading, for example, a blob object in a normal
/// way but only when needed.
pub struct BuferedFile {
    pub path: PathBuf,
    pub reader: BufReader<File>,
    pub cache: IndexEntryCache,
}
impl BuferedFile {
    /// Attempts to open a file and assign a `BufReader` to it.
    ///
    /// If the cache of a file could not be obtained, it is just assigned as the default
    /// `IndexEntryCache` value.
    pub fn try_from_path(path: PathBuf) -> Result<Self> {
        let file = std::fs::File::open(&path).context(format!("could not open file {:?}", path))?;
        let metadata = file
            .metadata()
            .context(format!("could not get file metadata {:?}", path));
        Ok(Self {
            path,
            reader: BufReader::new(file),
            cache: match metadata {
                Ok(m) => IndexEntryCache::try_from_metadata(m)
                    .context("could not get cache data from metadata")
                    .warn_unwrap_or_default(),
                Err(e) => {
                    log::warn!("{:?}", e);
                    IndexEntryCache::default()
                }
            },
        })
    }
}

/// Goes through `files`, trying to open the file and returning a `BuferedFile` value for each one.
///
/// Use this function if you plan on iterating over a large amount of files and you don't really
/// need to store their content.
pub fn read_bufered(files: Vec<PathBuf>) -> Result<Vec<BuferedFile>> {
    let mut bufered = Vec::with_capacity(files.len());
    for p in files {
        bufered.push(BuferedFile::try_from_path(p).context("could not create bufered file")?);
    }
    Ok(bufered)
}

/// Returns all the files inside `root` as a `BuferedFile`, entering subdirectories recursively.
///
/// The paths returned by this function are all absolute.
///
/// This function will ignore files present in .gitignore.
///
/// # Errors
///
/// This function will fail if a file could not be opened through the `std::fs::File::open`
/// function.
pub fn get_all_files_bufered(root: &Path) -> Result<Vec<BuferedFile>> {
    let all_files = get_all_paths(root).context(format!("could not get paths in {:?}", root))?;
    read_bufered(all_files).context("could not create bufered files")
}


/// Returns the path of all the files inside of `root`, entering subdirectories recursively.
///
/// The paths returned by this function are absolute.
///
/// This function will ignore files present in .gitignore.
///
/// # Errors
///
/// This function will fail if it could not read the files in the working tree.
pub fn get_all_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let ignored = gitignore::read_gitignore(root).context("could not get ignored files")?;

    let root_dir = std::fs::read_dir(root).context("could not get root directories")?;

    let mut path: PathBuf;
    for direntry in root_dir {
        path = direntry.context("could not get dir entry")?.path();

        if ignored.contains(&path) {
            continue;
        }

        if path.is_dir() {
            paths.extend(get_all_paths(&path)?);
        } else {
            paths.push(path);
        }
    }

    Ok(paths)
}

/// Returns all the paths of the files and subdirectories inside of `dir`.
///
/// This function does not consider ignored files.
///
/// # Errors
///
/// This function will fail if:
/// `dir` did not exist.
/// `dir` was not a directory.
/// Could not get the files inside of `dir`.
pub fn read_all_dir_paths(path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let entries = std::fs::read_dir(path).context("could not get directory entries")?;
    for e in entries {
        paths.push(e?.path());
    }
    Ok(paths)
}

/// Given a list of paths, it expands the paths that are directories into the paths inside of them,
/// removing the directories.
///
/// # Example
///
/// ```rust
/// let paths = vec!["dir", "file.txt"].map(PathBuf::from).collect();
///
/// let expanded = get_all_files_from_list(paths).unwrap();
///
/// /// The "dir" directory contains two files inside: "a" and "b".
///
/// assert!(expanded.contains(PathBuf::from("dir/a")));
/// assert!(expanded.contains(PathBuf::from("dir/b")));
/// assert!(!expanded.contains(PathBuf::from("dir")));
/// ```
#[allow(unused)]
pub fn expand_dirs_from_list(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut sub: Vec<PathBuf>;
    let mut expanded: Vec<PathBuf> = Vec::new();
    for path in paths {
        if path.is_dir() {
            sub =
                read_all_dir_paths(&path).context(format!("could not get {:?} subpaths", path))?;
            expanded.extend(expand_dirs_from_list(sub)?);
        } else {
            expanded.push(path);
        }
    }
    Ok(expanded)
}

/// Returns the files in `path` that are not inside a .gitignore file in the same directory.
///
/// # Errors
///
/// This function can fail if it couldn't get the files inside `path` or could not filter from the
/// gitignore.
#[allow(unused)]
pub fn read_not_ignored_paths(path: &Path) -> Result<Vec<PathBuf>> {
    let all_paths = read_all_dir_paths(path).context("could not read root directory entries")?;
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
