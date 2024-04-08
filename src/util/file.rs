use crate::fs;
use std::path::Path;

#[allow(dead_code)]
pub fn get_file_type(file_path: &str) -> Option<&str> {
    let path = Path::new(file_path);
    path.extension().and_then(|ext| ext.to_str())
}

#[allow(dead_code)]
pub fn file_type_from_meta_info(file_path: &str) -> anyhow::Result<String> {
    let mut metainfo_path = file_path.to_string();
    metainfo_path.push_str(".meta");
    let metadata = fs::load_metadata(&metainfo_path)?;
    Ok(metadata.file_type)
}
