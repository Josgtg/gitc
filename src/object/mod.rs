#[allow(clippy::module_inception)]
mod object;

mod blob;
mod tree;
mod commit;

pub use object::Object;
pub use blob::ExtendedBlob;
