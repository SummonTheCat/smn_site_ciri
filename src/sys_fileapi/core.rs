//! Core file‑API logic: no Hyper types here.

use std::path::PathBuf;
use tokio::{fs, io::AsyncWriteExt};
use multer::Field;
use serde::Serialize;

const UPLOAD_DIR: &str = "uploads";

#[derive(Serialize)]
pub struct UploadResponse {
    pub filename: String,
    pub url: String,
}

/// Save a single uploaded field to disk and return its URL.
pub async fn api_upload_field(
    mut field: Field<'_>,
    base_url: &str,
) -> Result<UploadResponse, Box<dyn std::error::Error + Send + Sync>> {
    let orig = field
        .file_name()
        .ok_or("Field has no filename")?;
    let filename = sanitize_filename::sanitize(orig);
    fs::create_dir_all(UPLOAD_DIR).await?;
    let path = PathBuf::from(UPLOAD_DIR).join(&filename);
    let mut file = fs::File::create(&path).await?;
    while let Some(chunk) = field.chunk().await? {
        file.write_all(&chunk).await?;
    }
    let url = format!("{}/files/{}", base_url, filename);
    Ok(UploadResponse { filename, url })
}

/// List all filenames in the upload directory.
pub async fn api_list_files() -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut dir = fs::read_dir(UPLOAD_DIR).await?;
    let mut names = Vec::new();
    while let Some(entry) = dir.next_entry().await? {
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    Ok(names)
}

/// Delete a file by name.
///
/// Return `Ok(true)` if deleted, `Ok(false)` if it didn’t exist.
pub async fn api_remove_file(
    filename: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let path = PathBuf::from(UPLOAD_DIR).join(filename);
    match fs::remove_file(&path).await {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(Box::new(e)),
    }
}
