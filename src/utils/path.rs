use std::path::{Path, PathBuf};
use std::ffi::{OsStr, OsString};

/// Removes the first component from a path.
///
/// # Returns
///
/// A tuple with the removed root in the first position or `None` if there wasn't any root, and the
/// stripped path on the second position.
///
/// # Examples
///
/// ```rust
/// let path = PathBuf::from("dir/subdir/file.txt");
/// let (root, stripped_path) = strip_root(path);
///
/// assert_eq!(root, Some(PathBuf::from("dir")));
/// assert_eq!(stripped_path, PathBuf::from("subdir/file.txt"));
/// ```
pub fn strip_root(path: PathBuf) -> (Option<PathBuf>, PathBuf) {
    let mut components = path.components();
    let root = components.next().map(|c| PathBuf::from(c.as_os_str()));
    (root, components.as_path().to_owned())
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

