#[allow(clippy::module_inception)]
mod object;

pub mod blob;
pub mod commit;
pub mod tree;

pub use object::Object;

pub const SPACE_BYTE: u8 = b' ';
pub const NULL_BYTE: u8 = b'\0';
