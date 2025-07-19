use std::path::Path;

use crate::Constants;

use super::{ExtensionEntry, Index, IndexEntry};

#[derive(Debug, Default)]
pub struct IndexBuilder {
    index: Index,
}

impl IndexBuilder {
    /// Returns an `IndexBuilder` with a default index.
    pub fn new() -> Self {
        let mut ib = Self::default();
        ib.index.version_number = Constants::INDEX_VERSION_NUMBER;
        ib
    }

    pub fn build(mut self) -> Index {
        self.index
            .entries
            .sort_by(|e1, e2| Path::cmp(e1.path(), e2.path()));
        self.index.entries_number = self.index.entries.len() as u32;
        self.index
    }

    #[allow(unused)]
    pub fn set_version(&mut self, version: u32) {
        self.index.version_number = version;
    }

    pub fn add_index_entry(&mut self, entry: IndexEntry) {
        self.index.entries.push(entry)
    }
    pub fn remove_index_entry_by_path(&mut self, path: &Path) -> Option<IndexEntry> {
        let position = self.index.entries.iter().position(|ie| ie.path() == path);
        Some(self.index.entries.swap_remove(position?))
    }

    #[allow(unused)]
    pub fn add_extension_entry(&mut self, entry: ExtensionEntry) {
        self.index.extensions.push(entry)
    }
}

impl From<Index> for IndexBuilder {
    /// Creates an `IndexBuilder` starting from an already starting index, allowing it to modify
    /// it.
    fn from(index: Index) -> Self {
        Self { index }
    }
}
