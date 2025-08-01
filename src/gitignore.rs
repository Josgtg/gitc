use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::error::WarnUnwrap;
use crate::{utils, Constants};

use crate::utils::path::relative_path;

/// Struct intended to be used for any operations related to the .gitignore file.
///
/// It stores a `HashSet` containing the paths to ignore. It's important to know this paths are all
/// *canonicalized paths*.
pub struct Gitignore {
    files: HashSet<PathBuf>,
}
impl Gitignore {
    /// Tries to add a file to the list of ignored files.
    ///
    /// # Errors
    ///
    /// This function will fail if the file did not exist or could not be canonicalized.
    pub fn add_file(&mut self, path: PathBuf) -> Result<()> {
        let mut cleaned = utils::path::clean_path(&path, false);

        let canon = match cleaned.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // failed to canonicalize as absolute path, we will try as relative
                cleaned = utils::path::clean_path(&path, true);
                cleaned
                    .canonicalize()
                    .context(format!("could not canonicalize path {:?}", cleaned))?
            }
        };

        self.files.insert(canon);
        Ok(())
    }

    /// Will check if a canonicalized version of `path` is included in the ignored files.
    pub fn contains(&self, path: &Path) -> bool {
        let canon = path.canonicalize().context(format!(
            "could not canonicalize path when checking if gitignore contained path {:?}",
            path
        )).warn();

        match canon {
            Ok(c) => self.files.contains(&c),
            Err(_) => false,
        }
    }
}
impl Default for Gitignore {
    /// This version of the `Gitignore` struct always has the .git folder as ignored path.
    fn default() -> Self {
        let mut gitignore = Self {
            files: HashSet::new(),
        };
        let _ = gitignore.add_file(Constants::repository_path()).warn();
        gitignore
    }
}

/// Reads a .gitignore file inside of `path`, returning a HashSet including all the files listed (read by line).
///
/// This function will skip files in the gitignore that have text that could not be interpreted as a `String`.
///
/// This function expects the .gitignore file to be in the root of the given path.
///
/// # Errors
///
/// This function will fail if the .gitignore file could not been opened.
pub fn read_gitignore(path: &Path) -> Result<Gitignore> {
    let mut gitignore = Gitignore::default();

    let gitignore_path = path.join(Constants::GITIGNORE_FILE_NAME);

    let gitignore_exists = std::fs::exists(&gitignore_path)
        .context("could not verify existance of gitignore file")
        .warn_unwrap_or_default();

    if !gitignore_exists {
        return Ok(gitignore);
    }

    let gitignore_file = File::open(gitignore_path).context("could not open gitignore file")?;

    let reader = BufReader::new(gitignore_file);
    let mut path;
    let _: Gitignore;
    for line in reader.lines().map_while(Result::ok) {
        path = PathBuf::from(line);
        _ = gitignore
            .add_file(path)
            .context("could not add file to gitignore")
            .warn();
    }

    Ok(gitignore)
}

/// Returns a list of files not in the .gitignore file (filters `paths_to_filter`).
///
/// This function looks for a .gitignore file directly inside of `path_to_look`.
///
/// It also checks every path in `paths_to_filter` as if it was relative to `path_to_look`.
///
/// # Errors
///
/// This function can fail if the .gitignore file could not be read.
#[allow(unused)]
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
