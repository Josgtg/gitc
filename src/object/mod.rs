#[allow(clippy::module_inception)]
mod object;

pub mod blob;
pub mod commit;
pub mod tree;

pub use object::Object;
