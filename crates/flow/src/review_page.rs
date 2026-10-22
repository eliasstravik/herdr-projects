//! The evidence page. A review publishes only the evidence its reviewer selected, behind one small, portable page
//! (review_page/render.rs). One small detached web server (127.0.0.1 only, started on demand, restarted if gone)
//! serves the pages; where Tailscale is installed the same server is also shared privately on the tailnet
//! (tailscale serve, never funnel). The same process is the web Board's front door (webui/).
pub mod render;
pub use render::{render_review_page, select_evidence, web_url, REVIEW_FILE};

use crate::json::*;
use crate::store::files::{artifacts_dir, bin, data_dir, pid_alive, proc_start, read_json, write_json};
use crate::store::tasks::Task;
use std::path::Path;
use std::sync::OnceLock;

fn serve_file() -> std::path::PathBuf { data_dir().join("serve.json") }
fn server_alive(info: &Value) -> bool { info.b("pid") && pid_alive(info.i("pid")) && (!info.b("start") || proc_start(info.i("pid")).as_deref() == Some(info.s("start"))) }
/// The page server that runs now ({pid, port}), or None.
pub fn server_info() -> Option<Value> { read_json(&serve_file()).filter(|i| server_alive(i) && i.b("port")) }
pub fn ensure_server() -> Value {
    if let Some(i) = server_info() { return i; }
    start_server(&bin())
}
/// Start the page server (`<prog> serve`), detached, and wait a little for it to say its port. It is waited for when
/// it ends: one that could not take its port, crashed or was told to go is no zombie of the keeper, which asks for it
/// every 30 seconds (#311).
pub fn start_server(prog: &Path) -> Value {
    let mut c = std::process::Command::new(prog);
    c.arg("serve").current_dir(data_dir()).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    crate::runner::detach(&mut c);
    let pid = crate::runner::start_reaped(&mut c) as i64;
    if pid == 0 { return Value::Null; }
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Some(i) = read_json(&serve_file()) { if i.i("pid") == pid && i.b("port") { return i; } }
    }
    read_json(&serve_file()).unwrap_or(Value::Null)
}
fn tailscale(a: &[&str]) -> Option<String> {
    let o = std::process::Command::new("tailscale").args(a).stdin(std::process::Stdio::null()).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}
/// Share the local server on the tailnet, on an https port nothing else uses. Existing shares are never replaced.
fn ensure_tailscale(info: &Value) -> Option<String> {
    let status: Value = tailscale(&["serve", "status", "--json"]).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let used: Vec<i64> = status.at("TCP").as_object().map(|o| o.keys().filter_map(|k| k.parse().ok()).collect()).unwrap_or_default();
    let me: Value = serde_json::from_str(&tailscale(&["status", "--json"])?).ok()?;
    let host = me.at("Self").s("DNSName").trim_end_matches('.').to_string();
    if host.is_empty() { return None; }
    let ts = info.at("ts");
    if ts.is_object() && ts.i("target") == info.i("port") && used.contains(&ts.i("port")) { return Some(format!("https://{host}:{}", ts.i("port"))); }
    if ts.b("port") && used.contains(&ts.i("port")) { tailscale(&["serve", &format!("--https={}", ts.i("port")), "off"]); } // ours, pointing at an old port
    let mut port = 8750;
    while used.contains(&port) { port += 1; }
    tailscale(&["serve", "--bg", &format!("--https={port}"), &format!("http://127.0.0.1:{}", info.i("port"))])?;
    let mut all = read_json(&serve_file()).and_then(|v| v.as_object().cloned()).unwrap_or_default();
    all.put("ts", json!({ "port": port, "target": info.i("port"), "host": host }));
    let _ = write_json(&serve_file(), &Value::Object(all));
    Some(format!("https://{host}:{port}"))
}
/// How links are shared is fixed: privately on your tailnet when Tailscale is installed here, else 127.0.0.1.
pub fn link_mode() -> &'static str {
    static MODE: OnceLock<&'static str> = OnceLock::new();
    MODE.get_or_init(|| {
        let has = std::process::Command::new("sh").args(["-c", "command -v tailscale"]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false);
        if std::env::var("HERDR_PROJECTS_LINKS").as_deref() == Ok("local") || !has { "local" } else { "tailscale" }
    })
}
/// Where a task's saved artifacts are served from. Falls back to local when Tailscale is absent or sharing fails.
pub fn artifact_base(id: &str) -> String {
    if std::env::var("HERDR_PROJECTS_NO_SERVER").as_deref() == Ok("1") { return String::new(); }
    let info = ensure_server();
    if !info.b("port") { return String::new(); }
    let local = format!("http://127.0.0.1:{}", info.i("port"));
    let base = if link_mode() == "tailscale" { ensure_tailscale(&info).unwrap_or(local) } else { local };
    format!("{base}/{id}")
}
/// Write the page for a review that selected evidence, and say where it is. Nothing for a review without any.
pub fn publish_review(enabled: bool, dir: &Path, listed: &Value, review: &Value, task: &Task, ctx: &Value) -> Option<Vec<Value>> {
    if !enabled { return Some(vec![]); }
    let evidence = select_evidence(listed, dir);
    if evidence.is_empty() { return Some(vec![]); } // no page is made for tiny changes with no useful evidence
    std::fs::write(dir.join(REVIEW_FILE), render_review_page(ctx, review, &evidence, dir)).ok()?;
    let root = artifact_base(task.s("id")); // link sharing stays with the board, not with reviewer-authored HTML
    let link = if root.is_empty() { dir.join(REVIEW_FILE).display().to_string() } else { format!("{root}/{REVIEW_FILE}") };
    Some(vec![json!({ "file": REVIEW_FILE, "caption": "Open review", "detail": "Work · verdict · selected evidence", "kind": "review", "link": link })])
}

// ---- `herdr-projects serve`: 127.0.0.1 only; serves nothing but artifacts folders ------------------------------------
fn kind_of(file: &Path) -> &'static str {
    match file.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
        Some("html") => "text/html; charset=utf-8", Some("png") => "image/png", Some("jpg") | Some("jpeg") => "image/jpeg", Some("gif") => "image/gif", Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml", Some("mp4") => "video/mp4", Some("webm") => "video/webm", Some("txt") | Some("md") | Some("log") => "text/plain; charset=utf-8",
        Some("json") => "application/json", Some("css") => "text/css", Some("js") => "text/javascript", Some("pdf") => "application/pdf", _ => "application/octet-stream",
    }
}
/// One artifact file, when the path names one: /<repo>/<n>/<file…>, nothing hidden, nothing outside the folder.
pub fn artifact_file(path: &str) -> Option<std::path::PathBuf> {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() < 3 || parts.iter().any(|p| *p == ".." || p.starts_with('.')) { return None; }
    let base = artifacts_dir(&format!("{}/{}", parts[0], parts[1]));
    let file = parts[2..].iter().fold(base.clone(), |p, x| p.join(x));
    (file.starts_with(&base) && file.is_file()).then_some(file)
}
pub fn serve_artifact(path: &str) -> crate::http::Response {
    use crate::http::Response;
    match artifact_file(path).and_then(|f| std::fs::read(&f).ok().map(|b| (f, b))) {
        Some((f, body)) => Response::new(200, kind_of(&f), body).with("cache-control", "no-store").with("x-content-type-options", "nosniff"),
        None => Response::text(404, "Not found"),
    }
}
/// The page server's process, which is also the front door of the herdr web UI while that plugin is installed: the
/// command line hands it the front door's loop (webui/door_loop.rs), a part built on top of this one.
pub fn serve_main(front_door: fn()) -> i32 {
    let prev = read_json(&serve_file()).unwrap_or(Value::Null);
    let Ok(listener) = crate::http::listen(prev.i("port") as u16) else { return 1 };
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    let pid = std::process::id() as i64;
    let mut all = prev.as_object().cloned().unwrap_or_default();
    all.put("pid", pid); all.put("start", proc_start(pid)); all.put("port", port);
    let _ = write_json(&serve_file(), &Value::Object(all));
    // The same process is the front door of the herdr web UI, while that plugin is installed (webui/door_loop.rs).
    front_door();
    crate::http::serve(listener, |req, _| Some(serve_artifact(&req.path)));
    0
}
