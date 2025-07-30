use std::{env, path::PathBuf};

pub struct Constants;

impl Constants {
    pub const REPOSITORY_FOLDER_NAME: &str = ".git";
    pub const OBJECTS_FOLDER_NAME: &str = "objects";
    pub const REFS_FOLDER_NAME: &str = "refs";
    pub const HEADS_FOLDER_NAME: &str = "heads";
    pub const INDEX_NAME: &str = "index";
    pub const HEAD_CONTENT_HEADER: &str = "ref: ";
    pub const DEFAULT_BRANCH_NAME: &str = "main";
    pub const HEAD_FILE_NAME: &str = "HEAD";
    pub const INDEX_VERSION_NUMBER: u32 = 2;
    pub const INDEX_HEADER_BINARY: u32 = u32::from_be_bytes(*b"DIRC");
    pub const GITIGNORE_FILE_NAME: &str = ".gitignore";

    /// The root folder of the repository
    pub fn working_tree_root_path() -> PathBuf {
        env::current_dir().expect("failed to get current dir")
    }

    /// The location of the .git folder
    pub fn repository_path() -> PathBuf {
        let mut path = Constants::working_tree_root_path();
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
        path.push(Constants::HEAD_FILE_NAME);
        path
    }

    pub fn index_path() -> PathBuf {
        let mut path = Constants::repository_path();
        path.push(Constants::INDEX_NAME);
        path
    }

    pub fn default_head_content() -> String {
        format!(
            "{}{}/{}/{}",
            Constants::HEAD_CONTENT_HEADER,
            Constants::REFS_FOLDER_NAME,
            Constants::HEADS_FOLDER_NAME,
            Constants::DEFAULT_BRANCH_NAME
        )
    }
}
