use std::slice::Iter;

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
        self.index.entries_number = self.index.entries.len() as u32;
        self.index
    }

    pub fn set_version(&mut self, version: u32) {
        self.index.version_number = version;
    }

    pub fn add_index_entry(&mut self, entry: IndexEntry) {
        self.index.entries.push(entry)
    }

    pub fn add_extension_entry(&mut self, entry: ExtensionEntry) {
        self.index.extensions.push(entry)
    }

    pub fn iter_index_entries(&self) -> Iter<IndexEntry> {
        self.index.entries.iter()
    }
}

impl From<Index> for IndexBuilder {
    /// Creates an `IndexBuilder` starting from an already starting index, allowing it to modify
    /// it.
    fn from(index: Index) -> Self {
        Self { index }
    }
}
