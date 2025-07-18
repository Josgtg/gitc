use std::path::PathBuf;

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
