//! Named board identities, repo scope, and the registered folder new tasks belong to.
use crate::board::state::State;
use crate::err::Res;
use crate::json::*;

#[derive(Clone, Default)]
pub struct View { pub name: String, pub repos: Option<Vec<String>> }
impl View {
    pub fn parse(flags: &Obj) -> Res<Self> {
        let repos = if flags.has("repo") {
            if !flags.at("repo").is_string() { crate::fail!("--repo needs a repo name or comma-separated names."); }
            let mut names = vec![];
            for name in flags.s("repo").split(',').map(str::trim) {
                if name.is_empty() { crate::fail!("--repo needs nonempty repo names."); }
                if !names.iter().any(|n| n == name) { names.push(name.to_string()); }
            }
            Some(names)
        } else { None };
        let name = if flags.has("view") {
            if !flags.at("view").is_string() { crate::fail!("--view needs a name."); }
            let name = flags.t("view");
            if !valid(&name) { crate::fail!("Invalid view name: use only a-z, 0-9, dot, underscore and hyphen."); }
            name
        } else if let Some(repos) = &repos {
            let slug: String = repos.join(",").to_lowercase().chars().map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c) { c } else { '-' }).collect();
            format!("repo-{slug}")
        } else { String::new() };
        Ok(Self { name, repos })
    }
}
fn valid(name: &str) -> bool { !name.is_empty() && name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"._-".contains(&c)) }
/// Exact repo names, shared by local and remote rows.
pub fn includes(s: &State, repo: &str) -> bool { s.repos.is_empty() || s.repos.iter().any(|n| n == repo) }
pub fn label(s: &State) -> String {
    if s.repos.is_empty() { String::new() } else { format!("only repo{} {}", if s.repos.len() == 1 { "" } else { "s" }, s.repos.join(", ")) }
}
/// No guessed paths: an unknown scope stays empty and cannot send work to the launch folder.
pub fn folder(s: &State) -> Res<String> {
    let Some(repo) = s.repos.first() else { return Ok(s.ctx.cwd.clone()) };
    let map = crate::store::repos::known_repos();
    match map.get(repo) {
        Some(p) if std::path::Path::new(&text(p)).is_dir() => Ok(text(p)),
        _ => crate::fail!("Unknown repo {repo}. Register it by adding a task from its folder first."),
    }
}
pub fn configure(s: &mut State, view: &View) {
    s.view_name = view.name.clone();
    if let Some(repos) = &view.repos { s.repos = repos.clone(); }
    if !s.repos.is_empty() {
        match folder(s) {
            Ok(cwd) => { let here = crate::board::data::repo_here(&cwd); s.ctx.cwd = cwd; s.data.repo = here.0; s.data.repo_root = here.1; }
            Err(e) => s.flash(e.msg, crate::shown::ink::Ink::Yellow),
        }
    }
}
