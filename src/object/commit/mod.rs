mod commit;
mod commit_user;

pub use commit::*;
pub use commit_user::*;

use time::format_description::BorrowedFormatItem;
use time::macros::format_description;

pub const COMMIT_STR: &str = "commit";
pub const TREE_STR: &str = "tree";
pub const PARENT_STR: &str = "parent";
pub const AUTHOR_STR: &str = "author";
pub const COMMITTER_STR: &str = "committer";
const TIMEZONE_FORMAT: &[BorrowedFormatItem] =
    format_description!("[offset_hour sign:mandatory][offset_minute]");
