use crate::{error::CustomResult, fs::index::read_index_file, Result};

pub fn ls_files(debug: bool) -> Result<String> {
    let index = read_index_file().map_err_with("could not read from index file")?;
    
    let mut formatted = String::new();
    for e in index.entries() {
        if debug {
            formatted.push_str(format!("{e:?}").as_str());
            formatted.push('\n');
        } else {
            formatted.push_str(format!("{e}").as_str());
        }
        formatted.push('\n');
    }
    formatted.pop();

    Ok(formatted.to_string())
}
