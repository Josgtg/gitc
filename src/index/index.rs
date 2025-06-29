use std::io::Cursor;
use std::io::Write;

use byteorder::BigEndian;
use byteorder::WriteBytesExt;

use crate::Constants;

use super::IndexEntry;
use super::ExtensionEntry;

pub struct Index {
    version_number: u32,
    entries_number: u32,
    entries: Vec<IndexEntry>,
    extensions: Vec<ExtensionEntry>,
    checksum: [u8; 20],
}

impl Index {
    pub fn new(
        version_number: u32,
        entries_number: u32,
        entries: Vec<IndexEntry>,
        extensions: Vec<ExtensionEntry>,
        checksum: [u8; 20],    
    ) -> Self {
        Self {
            version_number,
            entries_number,
            entries,
            extensions,
            checksum,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let bytes = Vec::with_capacity(32);

        let mut cursor = Cursor::new(bytes);

        cursor.write_all(Constants::INDEX_HEADER_BINARY).unwrap();
        cursor.write_u32::<BigEndian>(self.version_number).unwrap();
        cursor.write_u32::<BigEndian>(self.entries_number).unwrap();

        let mut entry_data: Vec<u8>;
        let mut offset: usize;
        let mut current_len: usize = 12;
        for e in self.entries.iter() {
            entry_data = e.to_bytes();
            cursor.write_all(&entry_data).unwrap();
            current_len += entry_data.len();
            offset = 8 - ((8 % current_len) % 8);
            for _ in 0..offset {
                cursor.write_u8(0).unwrap();
            }
        }

        for _e in self.extensions.iter() {

        }

        // assigning checksum from previous data

        cursor.write_all(&self.checksum).unwrap();

        cursor.into_inner()
    }
}
