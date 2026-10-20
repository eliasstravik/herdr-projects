//! Which host a repo's origin is at, from its URL: Azure DevOps (its organization, project and repo) or not. Every form
//! Azure DevOps gives a clone: `git@ssh.dev.azure.com:v3/<org>/<project>/<repo>`, `https://<org>@dev.azure.com/<org>/
//! <project>/_git/<repo>`, and the older `<org>.visualstudio.com` and `vs-ssh.visualstudio.com` ones.
use crate::store::repos::git_out;
use std::path::Path;

/// A repo at Azure DevOps. The names are as Azure DevOps says them (`My Project`, not `My%20Project`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Azure { pub org: String, pub project: String, pub repo: String }
impl Azure {
    /// What `az … --org` takes.
    pub fn org_url(&self) -> String { format!("https://dev.azure.com/{}", enc(&self.org)) }
    /// The repo's page.
    pub fn web(&self) -> String { format!("{}/{}/_git/{}", self.org_url(), enc(&self.project), enc(&self.repo)) }
    pub fn pr_url(&self, id: &str) -> String { format!("{}/pullrequest/{id}", self.web()) }
    pub fn build_url(&self, id: &str) -> String { format!("{}/{}/_build/results?buildId={id}", self.org_url(), enc(&self.project)) }
    /// A pull request in Azure DevOps' REST API, on the repository's route (the one that takes a PATCH).
    pub fn pr_api(&self, id: &str) -> String { format!("{}/{}/_apis/git/repositories/{}/pullRequests/{id}?api-version=7.1", self.org_url(), enc(&self.project), enc(&self.repo)) }
}
fn dec(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%' && i + 2 < b.len() { if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) { out.push((h * 16 + l) as u8); i += 3; continue; } }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}
fn enc(s: &str) -> String { s.chars().map(|c| if c.is_ascii_alphanumeric() || "-._~".contains(c) { c.to_string() } else { c.to_string().bytes().map(|b| format!("%{b:02X}")).collect() }).collect() }

/// The Azure DevOps repo an origin URL names. None: another host (GitHub, a local path).
pub fn parse(url: &str) -> Option<Azure> {
    let u = url.trim().trim_end_matches('/');
    let mut rest = u.split_once("://").map(|(_, r)| r).unwrap_or(u);
    // A user (`git@`, `<org>@`, `user:token@`) before the host is not part of it.
    let slash = rest.find('/').unwrap_or(rest.len());
    if let Some(at) = rest[..slash].rfind('@') { rest = &rest[at + 1..]; }
    let cut = rest.find([':', '/']).unwrap_or(rest.len());
    let host = rest[..cut].to_lowercase();
    let path: Vec<String> = rest[cut..].split(['/', ':']).filter(|p| !p.is_empty()).map(dec).collect();
    let p: Vec<&str> = path.iter().map(String::as_str).collect();
    let made = |org: &str, project: &str, repo: &str| (!org.is_empty() && !project.is_empty() && !repo.is_empty()).then(|| Azure { org: org.into(), project: project.into(), repo: repo.trim_end_matches(".git").into() });
    match (host.as_str(), p.as_slice()) {
        ("ssh.dev.azure.com" | "vs-ssh.visualstudio.com", ["v3", org, project, repo]) => made(org, project, repo),
        ("dev.azure.com", [org, project, "_git", repo]) => made(org, project, repo),
        // A repo named as its project.
        ("dev.azure.com", [org, "_git", repo]) => made(org, repo, repo),
        (h, rest) if h.ends_with(".visualstudio.com") && h != "vs-ssh.visualstudio.com" => {
            let org = h.trim_end_matches(".visualstudio.com");
            match rest {
                ["DefaultCollection", "_git", repo] | ["_git", repo] => made(org, repo, repo),
                ["DefaultCollection", project, "_git", repo] | [project, "_git", repo] => made(org, project, repo),
                _ => None,
            }
        }
        _ => None,
    }
}
/// The Azure DevOps repo a folder's origin is at, as its git config says it (never rewritten by `insteadOf`). None: the
/// origin is elsewhere (GitHub), or there is none.
pub fn host(root: &Path) -> Option<Azure> {
    let url = git_out(root, &["config", "--get", "remote.origin.url"]).ok()?;
    if !url.contains("azure.com") && !url.contains("visualstudio.com") { return None; }
    parse(&url)
}
