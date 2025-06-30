use crate::Constants;

use super::{ExtensionEntry, Index, IndexEntry};

#[derive(Debug, Default)]
pub struct IndexBuilder {
    index: Index,
    entries: Vec<IndexEntry>,
    extensions: Vec<ExtensionEntry>,
}

impl IndexBuilder {
    
    /// Returns an `IndexBuilder` with a default index.
    pub fn new() -> Self {
        let mut ib = Self::default();
        ib.index.version_number = Constants::INDEX_VERSION_NUMBER;
        ib
    }

    pub fn build(mut self) -> Index {
        self.index.entries_number = self.entries.len() as u32;
        self.index.entries = self.entries.into();
        self.index.extensions = self.extensions.into();
        self.index
    }

    pub fn set_version(&mut self, version: u32) {
        self.index.version_number = version;
    }

    pub fn add_index_entry(&mut self, entry: IndexEntry) {
        self.entries.push(entry)
    }

    pub fn add_extension_entry(&mut self, entry: ExtensionEntry) {
        self.extensions.push(entry)
    }
}

