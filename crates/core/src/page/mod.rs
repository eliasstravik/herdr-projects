//! The readable page the board publishes beside a source file (a plan, a report, a closing note).
//!
//! Agents write Markdown; the board makes one portable `index.html` next to it, so the human gets a link that opens
//! as a page in every browser instead of raw text. The page is generated, not authored: it carries a marker, is
//! regenerated when its source changes, and never replaces an `index.html` the agent wrote itself. Raw HTML in the
//! Markdown is escaped, links are limited to http(s), mailto, anchors and files of the same task, and no script or
//! external resource is loaded. Evidence files are linked only when they are plain, small text that passes the
//! obvious-secret check; nothing is invented when there is none.
use std::fs;
use std::path::{Path, PathBuf};

pub const GENERATED: &str = "herdr-projects page";
const MARKER: &str = r#"<meta name="generator" content="herdr-projects page">"#;
const MAX_EVIDENCE: u64 = 512 * 1024;

/// The kind of page, in the small banner above its content.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind { Report, Plan, Closing }

impl Kind {
    fn word(self) -> &'static str { match self { Kind::Report => "Report", Kind::Plan => "Plan", Kind::Closing => "Closing summary" } }
    fn note(self) -> &'static str {
        match self {
            Kind::Plan => "This is a plan or a static design, not the built implementation. Publishing this page does not approve it: the work board records approval separately, and nothing is implemented before that yes.",
            Kind::Report => "Published by the board from the source below. The Markdown file is the record; this page is its readable view.",
            Kind::Closing => "The closing summary the board kept for this task. The Markdown file is the record; this page is its readable view.",
        }
    }
}

/// Is this `index.html` one the board made (and may replace), rather than an agent's own page?
pub fn generated(file: &Path) -> bool {
    let Ok(bytes) = fs::read(file) else { return false };
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(8192)]);
    head.contains(MARKER)
}

/// A name the board will not publish as evidence: hidden files, keys, credentials and whole transcripts.
fn private_name(name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with('.') || ["secret", "token", "credential", "password", "passwd", "private-key", "id_rsa", "id_ed25519", "api-key", "apikey", "auth.json", "transcript", "conversation"].iter().any(|x| n.contains(x))
        || [".pem", ".key", ".p12", ".pfx", ".jks", ".env"].iter().any(|x| n.ends_with(x))
}
/// Obvious credentials in the first part of a text file. A guard against a careless paste, not a proof of privacy.
fn looks_secret(text: &str) -> bool {
    let t = text.to_lowercase();
    ["ghp_", "github_pat_", "gho_", "ghs_", "glpat-", "xoxb-", "xoxp-", "akia", "-----begin rsa private key", "-----begin openssh private key", "-----begin ec private key", "authorization: bearer", "password=", "passwd=", "secret_key=", "api_key="]
        .iter().any(|x| t.contains(x))
}
/// The evidence files in a task's page folder that may be linked: plain `.txt`/`.log`, small, not private.
pub fn evidence_files(dir: &Path) -> Vec<String> {
    let Ok(rd) = fs::read_dir(dir) else { return vec![] };
    let mut names: Vec<String> = rd.filter_map(|e| e.ok()).filter_map(|e| {
        let name = e.file_name().to_string_lossy().into_owned();
        let lower = name.to_lowercase();
        if name == "index.html" || private_name(&name) || !(lower.ends_with(".txt") || lower.ends_with(".log")) { return None; }
        let meta = e.metadata().ok()?;
        if !meta.is_file() || meta.len() == 0 || meta.len() > MAX_EVIDENCE { return None; }
        let mut buf = Vec::new();
        use std::io::Read;
        fs::File::open(e.path()).and_then(|f| f.take(MAX_EVIDENCE).read_to_end(&mut buf)).ok()?;
        let head = String::from_utf8_lossy(&buf);
        if head.contains('\0') || looks_secret(&head) { return None; }
        Some(name)
    }).collect();
    names.sort();
    names
}

/// Give the source a page, unless the agent wrote its own. Returns the entry file (always `index.html`).
pub fn ensure_page(dir: &Path, source: &str, title: &str, kind: Kind) -> Option<PathBuf> {
    let src = if Path::new(source).is_absolute() { PathBuf::from(source) } else { dir.join(source) };
    if !src.is_file() { return None; }
    let target = dir.join("index.html");
    if target.is_file() && !generated(&target) { return Some(target); } // the agent's own page is kept
    let body = fs::read_to_string(&src).ok()?;
    let evidence = evidence_files(dir);
    let html = html::render(title, source, kind, &body, &evidence);
    if fs::read_to_string(&target).ok().as_deref() != Some(html.as_str()) { crate::store::files::write_atomic(&target, html.as_bytes()).ok()?; }
    Some(target)
}

/// The page's own small HTML helpers, shared by the Markdown renderer and the page shell.
pub(super) fn esc(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;") }
pub(super) fn attr(s: &str) -> String { esc(s) }
pub(super) fn url_encode(s: &str) -> String { crate::util::url_encode(s) }
/// A relative link that keeps its path separators (`../closing-note.md` is not `..%2Fclosing-note.md`).
pub(super) fn href_path(s: &str) -> String {
    if s.starts_with('#') { return s.to_string(); }
    s.split('/').map(|p| if p == ".." || p == "." { p.to_string() } else { url_encode(p) }).collect::<Vec<_>>().join("/")
}
mod html;
mod markdown;
#[cfg(test)]
mod tests;
