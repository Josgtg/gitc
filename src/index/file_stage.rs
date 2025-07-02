use crate::Error;

/// Represents a file stage, mainly related to a merge.
#[repr(u16)]
#[derive(Debug)]
pub enum FileStage {
    /// File is tracked and staged normally.
    Normal = 0,
    /// Version from the current branch.
    Ours = 1,
    /// Version from the branch being merged in.
    Theirs = 2,
    /// Common ancestor version during a merge.
    Base = 3,
}

impl TryFrom<u16> for FileStage {
    type Error = crate::Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Ours),
            2 => Ok(Self::Theirs),
            3 => Ok(Self::Base),
            _ => Err(Error::Generic(format!("value passed ({}) does not correspond to a file stage", value).into())),
        }
    }
}
