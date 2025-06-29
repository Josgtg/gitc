use crate::Result;

pub fn checkout(reference: &str) -> Result<String> {
    Ok(format!("HEAD is now at {reference}"))
}
