#[allow(clippy::module_inception)]
mod object;

mod blob;
mod commit;
mod tree;

pub use blob::ExtendedBlob;
pub use object::Object;
