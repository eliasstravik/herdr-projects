//! Local project folders shown in Mission Control's Developer sidebar, with their real Git origin.
use crate::store::files::user_home;
use crate::store::repos::git_out;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Project { pub name: String, pub path: PathBuf, pub remote: String }
pub fn root() -> PathBuf { std::env::var("HERDR_CONTROL_PROJECTS_DIR").map(PathBuf::from).unwrap_or_else(|_| user_home().join("Developer")) }
pub fn scan(root: &Path) -> std::io::Result<Vec<Project>> {
    let mut projects = vec![];
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !entry.file_type()?.is_dir() || name.starts_with('.') || name == "Archiv" { continue; }
        let path = entry.path();
        let remote = if path.join(".git").exists() { git_out(&path, &["remote", "get-url", "origin"]).unwrap_or_default() } else { String::new() };
        projects.push(Project { name, path, remote });
    }
    projects.sort_by_key(|p| p.name.to_lowercase());
    Ok(projects)
}
