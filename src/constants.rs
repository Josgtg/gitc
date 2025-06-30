use std::{env, path::PathBuf};

pub struct Constants;

impl Constants {
    pub const REPOSITORY_FOLDER_NAME: &'static str = ".git";
    pub const OBJECTS_FOLDER_NAME: &'static str = "objects";
    pub const REFS_FOLDER_NAME: &'static str = "refs";
    pub const HEADS_FOLDER_NAME: &'static str = "heads";
    pub const INDEX_NAME: &'static str = "index";
    pub const DEFAULT_HEAD: &'static str = "ref: refs/heads/main";
    pub const HEAD_NAME: &'static str = "HEAD";
    pub const INDEX_VERSION_NUMBER: u32 = 2;
    pub const INDEX_HEADER_BINARY: u32 = u32::from_be_bytes(*b"DIRC");
    pub const GITIGNORE_FILE_NAME: &'static str = ".gitignore";

    /// The root folder of the repository
    pub fn repository_folder_path() -> PathBuf {
        let current_dir = env::current_dir().expect("failed to get current dir");
        let mut path: PathBuf = PathBuf::from(current_dir);
        #[cfg(debug_assertions)]
        {
            path.push("test-repo");
        }
        path
    }

    /// The location of the .git folder
    pub fn repository_path() -> PathBuf {
        let mut path = Constants::repository_folder_path();
        path.push(Constants::REPOSITORY_FOLDER_NAME);
        path
    }

    pub fn objects_path() -> PathBuf {
        let mut path = Constants::repository_path();
        path.push(Constants::OBJECTS_FOLDER_NAME);
        path
    }

    pub fn refs_path() -> PathBuf {
        let mut path = Constants::repository_path();
        path.push(Constants::REFS_FOLDER_NAME);
        path
    }

    pub fn heads_path() -> PathBuf {
        let mut path = Constants::refs_path();
        path.push(Constants::HEADS_FOLDER_NAME);
        path
    }

    pub fn head_path() -> PathBuf {
        let mut path = Constants::repository_path();
        path.push(Constants::HEAD_NAME);
        path
    }

    pub fn index_path() -> PathBuf {
        let mut path = Constants::repository_path();
        path.push(Constants::INDEX_NAME);
        path
    }
}
