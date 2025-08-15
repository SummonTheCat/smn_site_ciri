use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

/// Link entry inside project info
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProjectLink {
    pub link: String,
    pub description: String,
}

/// Project info JSON structure
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProjectInfo {
    pub project_name: String,
    pub project_description: String,
    pub project_state: String,
    #[serde(default)]
    pub project_tools: Vec<String>,
    #[serde(default)]
    pub project_images: Vec<String>, // now used AS-IS in HTML (absolute/relative per JSON)
    #[serde(default)]
    pub project_videos: Vec<String>, // now used AS-IS in HTML
    /// Path to markdown file (RELATIVE to the project directory that contains the JSON file)
    #[serde(default)]
    pub project_content: String,
    #[serde(default)]
    pub project_links: Vec<ProjectLink>,
}

/// Simple native error type
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self { Error::Io(e) }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self { Error::Json(e) }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Json(e) => write!(f, "JSON parse error: {e}"),
        }
    }
}
impl std::error::Error for Error {}

/// Build absolute FS path to the project directory:
/// data/projectData/<url_relative>/
pub fn project_dir_for<P: AsRef<Path>>(base_data_dir: P, url_relative: &str) -> PathBuf {
    let rel = url_relative.trim_start_matches('/');
    base_data_dir.as_ref().join(rel)
}

/// Load project info from one of these in the project dir (in order):
/// - projectData.json   (new primary)
/// - projectInfo.json
/// - project.json
/// - projectdata.json   (legacy)
pub fn get_project_info<P: AsRef<Path>>(base_data_dir: P, url_relative: &str) -> Result<ProjectInfo, Error> {
    let proj_dir = project_dir_for(base_data_dir, url_relative);
    let candidates = ["projectData.json", "projectInfo.json", "project.json", "projectdata.json"];

    for name in candidates {
        let p = proj_dir.join(name);
        if p.is_file() {
            let file = File::open(&p)?;
            let reader = BufReader::new(file);
            let info: ProjectInfo = serde_json::from_reader(reader)?;
            return Ok(info);
        }
    }

    Err(Error::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "project info not found",
    )))
}

/// Load markdown from the PROJECT DIRECTORY (same directory as the JSON),
/// NOT from a "resources" subdirectory anymore.
/// If `project_content` is empty, returns Ok("").
pub fn load_markdown_content<P: AsRef<Path>>(
    base_data_dir: P,
    url_relative: &str,
    md_rel_path: &str,
) -> Result<String, Error> {
    if md_rel_path.trim().is_empty() {
        return Ok(String::new());
    }
    let proj_dir = project_dir_for(base_data_dir, url_relative);
    let path = proj_dir.join(md_rel_path);
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}
