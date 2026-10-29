//! Plans, reports and closing notes as the human receives them: a readable `index.html` published by the board, its
//! Markdown source and sanitised evidence linked beside it, and the real link (never a guessed one) in the board's
//! words and details. Also the two cases the human reported: MCP-13's plan kept only as `plan.md`, and MCP-14's report
//! `index.html` at the root of its report folder, written before a failed send and never registered.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::research::{keep_report, report_folder, report_words, report_view_with};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

fn put(id: &str, k: &str, v: Value) { sw::store::tasks::patch(id, |t| { t.put(k, v); }).unwrap(); }
fn write(id: &str, name: &str, body: &str) { let d = report_folder(&obj(json!({ "id": id }))); std::fs::create_dir_all(&d).unwrap(); std::fs::write(d.join(name), body).unwrap(); }
fn read_file(p: &Path) -> String { std::fs::read_to_string(p).unwrap() }

/// An in-process page server, as `herdr-projects serve` runs it, on a free loopback port.
fn page_server() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || sw::http::serve(l, |req, _| Some(sw::review_page::serve_artifact(&req.path))));
    port
}
/// What the page server wrote down: this process, this port, and optionally the private Tailscale share.
fn serve_json(port: u16, ts: Option<(&str, u16)>) {
    let mut m = sw::store::files::marker();
    m.put("port", json!(port));
    if let Some((host, https)) = ts { m.put("ts", json!({ "host": host, "port": https, "target": port })); }
    sw::store::files::write_json(&sw::store::files::data_dir().join("serve.json"), &Value::Object(m)).unwrap();
}
fn get(port: u16, path: &str) -> (u16, String, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n").unwrap();
    let mut out = String::new(); let _ = s.read_to_string(&mut out);
    let (head, body) = out.split_once("\r\n\r\n").unwrap_or((&out, ""));
    (head.split(' ').nth(1).and_then(|x| x.parse().ok()).unwrap_or(0), head.to_string(), body.to_string())
}
/// A stand-in `tailscale` for the private-share path, as the real one reports it: the host and the HTTPS port in use.
fn fake_tailscale(base: &Path, host: &str, used: u16) {
    let dir = base.join("tailscale-bin"); std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("tailscale");
    std::fs::write(&script, format!("#!/bin/sh\nif [ \"$*\" = \"serve status --json\" ]; then printf '{{\"TCP\":{{\"{used}\":{{\"HTTPS\":true}}}}}}'; exit 0; fi\nif [ \"$*\" = \"status --json\" ]; then printf '{{\"Self\":{{\"DNSName\":\"{host}.\"}}}}'; exit 0; fi\nexit 0\n")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let old = std::env::var("PATH").unwrap_or_default();
    unsafe { std::env::set_var("PATH", format!("{}:{old}", dir.display())) };
}
/// The board's own environment is shared by every test: put the page server back off when this one ends.
struct ServerOn { old_path: Option<String> }
impl ServerOn {
    fn new(port: u16, ts: Option<(&str, u16)>, base: Option<&Path>) -> Self {
        unsafe { std::env::remove_var("HERDR_PROJECTS_NO_SERVER"); std::env::set_var("HERDR_PROJECTS_LINKS", if ts.is_some() { "tailscale" } else { "local" }); }
        if let (Some((host, used)), Some(base)) = (ts, base) { fake_tailscale(base, host, used); }
        serve_json(port, ts);
        ServerOn { old_path: std::env::var("PATH").ok() }
    }
}
impl Drop for ServerOn {
    fn drop(&mut self) {
        unsafe { std::env::set_var("HERDR_PROJECTS_NO_SERVER", "1"); std::env::set_var("HERDR_PROJECTS_LINKS", "local"); }
        if let Some(p) = &self.old_path { unsafe { std::env::set_var("PATH", p); } }
        let _ = std::fs::remove_file(sw::store::files::data_dir().join("serve.json"));
    }
}

#[test]
fn mcp13_a_plan_only_on_disk_gets_a_published_page_with_its_markdown_and_evidence_but_is_not_approved() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let id = w.add("Plan the interface");
    let plan_dir = sw::plan::plan_folder(&task(&id));
    std::fs::create_dir_all(&plan_dir).unwrap();
    std::fs::write(plan_dir.join("plan.md"), "# Plan\n\n1. Add f.txt.\n\n```html\n<button>Static sketch</button>\n```\n").unwrap();
    std::fs::write(plan_dir.join("evidence.txt"), "check: 12 passed\n").unwrap();
    let link = sw::plan::plan_link(&task(&id), || format!("https://box.ts.net:8751/{id}"));
    assert_eq!(link, format!("https://box.ts.net:8751/{id}/plan/index.html"), "the entry page, not plan.md");
    let page = read_file(&plan_dir.join("index.html"));
    assert!(page.contains("Source Markdown (plan.md)") && page.contains("./plan.md"), "{page}");
    assert!(page.contains("evidence.txt") && page.contains("Static design sketch") && page.contains("&lt;button&gt;"), "{page}");
    assert!(page.contains("Publishing this page does not approve it"), "{page}");
    assert_eq!((task(&id).s("stage"), task(&id).at("plan").b("approvedAt")), ("todo", false), "publishing is not approval");
    // A revised plan regenerates the same page; the link is stable and the old text is gone.
    sw::plan::write_plan(&task(&id), "# Plan v2\n\n1. Add g.txt.\n").unwrap();
    let page2 = read_file(&plan_dir.join("index.html"));
    assert!(page2.contains("Plan v2") && !page2.contains("Add f.txt") && page2.contains("plan.md"));
    assert_eq!(sw::plan::plan_link(&task(&id), || format!("https://box.ts.net:8751/{id}")), link);
}

#[test]
fn mcp14_a_report_index_at_the_root_is_discovered_and_served_after_a_failed_send_without_touching_the_task() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let id = w.add("Restore the checkout");
    put(&id, "stage", json!("failed")); put(&id, "msg", json!("Not started: its instructions sit unsent in its box"));
    put(&id, "failure", json!({ "stage": "send", "title": "The agent has not started on its instructions", "detail": "Informe completo: /home/me/.herdr-projects/reports/proj/1/index.html", "agentCanFix": false }));
    let dir = report_folder(&task(&id));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<!doctype html><html><head><title>MCP-14</title></head><body><h1>Restored</h1></body></html>").unwrap();
    let port = page_server();
    let _on = ServerOn::new(port, Some(("construct.example.ts.net", 8756)), Some(&w.base));
    // `details` must stop saying report:null: it resolves the real page and the real server address.
    let d = sw::actions::finish_line::details(&id).unwrap();
    let link = d.at("report").s("link");
    assert_eq!(link, format!("https://construct.example.ts.net:8756/{id}/report/index.html"), "{d}");
    assert_eq!(d.at("report").s("file"), "index.html");
    assert_eq!(d.at("report").at("source"), &Value::Null, "there is no Markdown source to invent");
    assert_eq!(sw::store::tasks::get(&id).unwrap().at("report"), &Value::Null, "the task is not rewritten");
    // The board row and the open-link key see the same discovered page as `details`.
    let rows = sw::flow::live::board(false);
    let row = rows.iter().find(|r| r.s("id") == id).unwrap_or_else(|| panic!("no row for {id}"));
    assert_eq!(row.at("report").s("link"), link);
    assert_eq!(sw::board::update::link::of(row), Some(("report", link.to_string())));
    let (st, head, body) = get(port, &format!("/{id}/report/index.html"));
    assert_eq!(st, 200, "{head}"); assert!(head.contains("content-type: text/html") && head.contains("content-security-policy"), "{head}");
    assert!(body.contains("<h1>Restored</h1>"), "{body}");
    let (st, head, _) = get(port, &format!("/{id}/report/"));
    assert_eq!(st, 302); assert!(head.contains(&format!("location: /{id}/report/index.html")), "{head}");
    let (st, head, _) = get(port, &format!("/{id}/report/missing.txt"));
    assert_eq!(st, 404); assert!(head.contains("text/plain"), "{head}");
}

#[test]
fn a_report_markdown_becomes_the_entry_page_with_its_source_and_only_safe_evidence() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let id = w.add("Look into it");
    write(&id, "report.md", "# What I found\n\nA table:\n\n| A | B |\n|---|---|\n| 1 | 2 |\n");
    write(&id, "evidence.txt", "tests: 12 passed\n");
    write(&id, "token.txt", "ghp_notpublished\n");
    let base = format!("https://box.ts.net:8750/{id}");
    let kept = keep_report(&task(&id), || base.clone()).unwrap();
    assert_eq!(kept.s("file"), "index.html"); assert_eq!(kept.s("link"), format!("{base}/report/index.html"));
    let page = read_file(&report_folder(&task(&id)).join("index.html"));
    assert!(page.contains("<h1 id=\"what-i-found\">What I found</h1>") && page.contains("<table>"), "{page}");
    assert!(page.contains("Source Markdown (report.md)") && page.contains("evidence.txt"), "{page}");
    assert!(!page.contains("token.txt") && !page.contains("ghp_notpublished"), "private evidence never linked: {page}");
    put(&id, "report", kept);
    let words = report_words(&task(&id));
    assert!(words.contains(&format!("[Open report]({base}/report/index.html)")) && words.contains(&format!("direct link: {base}/report/index.html")), "{words}");
    // The page, its Markdown source and the sanitised TXT all answer over HTTP, with the right kinds.
    let port = page_server();
    let _on = ServerOn::new(port, None, None);
    for (path, kind) in [(format!("/{id}/report/index.html"), "text/html"), (format!("/{id}/report/report.md"), "text/markdown"), (format!("/{id}/report/evidence.txt"), "text/plain")] {
        let (st, head, body) = get(port, &path);
        assert_eq!(st, 200, "{path}: {head}"); assert!(head.contains(kind), "{path}: {head}"); assert!(!body.is_empty(), "{path}");
    }
    assert_eq!(get(port, &format!("/{id}/report/")).0, 302);
    assert_eq!(get(port, &format!("/{id}/report/token.txt")).0, 404, "private evidence is not served");
}

#[test]
fn an_old_markdown_link_is_presented_as_the_published_entry_page() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let id = w.add("Old report");
    write(&id, "report.md", "# Old\n");
    put(&id, "report", json!({ "file": "report.md", "link": format!("https://box.ts.net:8750/{id}/report/report.md"), "at": sw::clock::now() }));
    let d = sw::actions::finish_line::details(&id).unwrap();
    assert_eq!(d.at("report").s("file"), "index.html");
    assert_eq!(d.at("report").s("link"), format!("https://box.ts.net:8750/{id}/report/index.html"));
    assert_eq!(task(&id).at("report").s("file"), "report.md", "presentation is repaired, the task record is not rewritten by a look");
}

#[test]
fn without_a_page_server_the_board_says_not_published_and_never_invents_a_url() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let id = w.add("Look into it");
    write(&id, "report.md", "# Report\n");
    let kept = keep_report(&task(&id), String::new).unwrap();
    assert_eq!(kept.s("link"), report_folder(&task(&id)).join("index.html").display().to_string());
    put(&id, "report", kept);
    let words = report_words(&task(&id));
    assert!(words.contains("not published") && !words.contains("http://") && !words.contains("https://"), "{words}");
    assert_eq!(report_words(&task(&w.add("Nothing written"))), "No report was found.");
    let view = report_view_with(&task(&w.add("Nothing written")), String::new);
    assert!(view.is_none(), "no report, no invented page");
}

#[test]
fn a_closing_note_gets_a_page_only_when_the_task_is_done() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let id = w.add("Finish it");
    let note = sw::flow::task::note_file(&task(&id));
    std::fs::write(&note, "# Closing\n\nAll done.\n").unwrap();
    put(&id, "stage", json!("failed"));
    let failed = sw::flow::task::closing_view(&task(&id), || format!("https://box.ts.net:8751/{id}")).unwrap();
    assert_eq!(failed.s("link"), format!("https://box.ts.net:8751/{id}/closing-note.md"), "honest, not a page that does not exist");
    put(&id, "stage", json!("done"));
    let done = sw::flow::task::closing_view(&task(&id), || format!("https://box.ts.net:8751/{id}")).unwrap();
    assert_eq!(done.s("file"), "index.html"); assert_eq!(done.s("link"), format!("https://box.ts.net:8751/{id}/closing/index.html"));
    assert!(read_file(&sw::store::files::dir_of(&id).join("artifacts/closing/index.html")).contains("All done."));
}

#[test]
fn the_servers_real_host_and_port_are_kept_for_each_instance() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let id = w.add("Link me");
    let dir = report_folder(&task(&id)); std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<!doctype html><title>Report</title>").unwrap();
    let port = page_server();
    let _on = ServerOn::new(port, Some(("construct.example.ts.net", 8751)), Some(&w.base));
    assert_eq!(sw::review_page::public_root(), "https://construct.example.ts.net:8751");
    let first = report_view_with(&task(&id), || format!("{}/{id}", sw::review_page::public_root())).unwrap();
    assert_eq!(first.s("link"), format!("https://construct.example.ts.net:8751/{id}/report/index.html"));
    serve_json(port, Some(("other.example.ts.net", 9001)));
    assert_eq!(sw::review_page::public_root(), "https://other.example.ts.net:9001");
    let second = report_view_with(&task(&id), || format!("{}/{id}", sw::review_page::public_root())).unwrap();
    assert_eq!(second.s("link"), format!("https://other.example.ts.net:9001/{id}/report/index.html"));
    serve_json(port, None);
    assert_eq!(sw::review_page::public_root(), format!("http://127.0.0.1:{port}"), "no private share: loopback, never a guessed host");
}

#[test]
fn no_task_folder_leaks_a_private_file_or_a_directory_listing() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let id = w.add("Keep private");
    let art = sw::store::files::artifacts_dir(&id); std::fs::create_dir_all(&art).unwrap();
    std::fs::write(sw::store::files::dir_of(&id).join("tokens.json"), "secret").unwrap();
    std::fs::write(art.join("token.txt"), "ghp_secret").unwrap();
    std::fs::write(art.join("notes.txt"), "fine").unwrap();
    std::os::unix::fs::symlink(sw::store::files::dir_of(&id).join("tokens.json"), art.join("stolen.txt")).unwrap();
    assert!(sw::review_page::artifact_file(&format!("{id}/token.txt")).is_none());
    assert!(sw::review_page::artifact_file(&format!("{id}/stolen.txt")).is_none());
    assert!(sw::review_page::artifact_file(&format!("{id}/notes.txt")).is_some());
    assert!(sw::review_page::artifact_file(&format!("{id}/../tokens.json")).is_none());
    let port = page_server();
    let _on = ServerOn::new(port, None, None);
    assert_eq!(get(port, &format!("/{id}/")).0, 404, "no directory listing");
}
