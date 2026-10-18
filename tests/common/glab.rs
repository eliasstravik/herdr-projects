//! A stand-in for GitLab as glab reaches it (`glab api`, flow::gitlab::set_fake_glab), answering in the test's own
//! thread: one merge request, its pipeline's jobs, a job's log. Every call is logged (`lab_calls`). No real glab, no network.
use super::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use sw::flow::git::GhOut;
use sw::flow::gitlab;
use sw::json::*;

/// What the fake GitLab holds: its one merge request, the jobs of its pipeline, a job's log, every call, and the commit a
/// rebase of the merge request puts it on (`rebase_to`; none: it stays where it is).
#[derive(Default)]
pub struct Lab { pub mr: Value, pub jobs: Vec<Value>, pub trace: String, pub log: Vec<String>, pub rebase_to: String }
pub const MR: &str = "https://gitlab.com/o/r/-/merge_requests/3";
fn ok(v: &Value) -> GhOut { GhOut { status: 0, stdout: if let Some(s) = v.as_str() { s.to_string() } else { v.to_string() }, stderr: String::new() } }
fn field<'a>(a: &'a [String], k: &str) -> Option<&'a str> { a.iter().find_map(|x| x.strip_prefix(&format!("{k}="))) }
fn answer(lab: &mut Lab, bare: &Path, a: &[String]) -> GhOut {
    lab.log.push(a.join(" "));
    let method = a.iter().position(|x| x == "-X").and_then(|i| a.get(i + 1)).map(String::as_str).unwrap_or("GET");
    let path = a.iter().skip(1).find(|x| x.contains('/') || *x == "user").cloned().unwrap_or_default();
    let head = |b: &str| git(bare, &["rev-parse", &format!("refs/heads/{b}")]);
    match (method, path.split('?').next().unwrap_or("")) {
        ("GET", "user") => ok(&json!({ "username": "me" })),
        ("GET", "projects/:id") => ok(&json!({ "default_branch": "main", "archived": false })),
        ("POST", "projects/:id/merge_requests") => {
            lab.mr = json!({ "iid": 3, "web_url": MR, "state": "opened", "title": field(a, "title"), "source_branch": field(a, "source_branch"), "target_branch": field(a, "target_branch"), "author": { "username": "me" } });
            ok(&lab.mr)
        }
        ("GET", "projects/:id/merge_requests") => ok(&json!(if lab.mr.is_object() && path.contains(&format!("source_branch={}", sw::util::url_encode(lab.mr.s("source_branch")))) { vec![lab.mr.clone()] } else { vec![] })),
        ("GET", "projects/:id/merge_requests/3") => {
            let mut mr = lab.mr.clone();
            // As GitLab does: right after it was asked, the rebase has not started yet (not in progress, the old commit).
            let started = lab.mr.i("rebaseLooks") > 0;
            if lab.mr.b("rebased") { lab.mr["rebaseLooks"] = json!(lab.mr.i("rebaseLooks") + 1); }
            mr["sha"] = json!(if lab.mr.b("rebased") && started { lab.rebase_to.clone() } else { head(lab.mr.s("source_branch")) });
            mr["diverged_commits_count"] = json!(if lab.rebase_to.is_empty() || lab.mr.b("rebased") { 0 } else { 1 });
            let st = |x: &str| lab.jobs.iter().any(|j| j.s("status") == x);
            let status = if st("failed") { "failed" } else if st("running") { "running" } else { "success" };
            if !lab.jobs.is_empty() { mr["head_pipeline"] = json!({ "id": 9, "sha": mr["sha"], "status": status, "web_url": "https://gitlab.com/o/r/-/pipelines/9" }); }
            ok(&mr)
        }
        ("PUT", "projects/:id/merge_requests/3") => { lab.mr["title"] = json!(field(a, "title")); ok(&lab.mr) }
        ("PUT", "projects/:id/merge_requests/3/rebase") => { if !lab.rebase_to.is_empty() { lab.mr["rebased"] = json!(true); } ok(&json!({ "rebase_in_progress": true })) }
        ("PUT", "projects/:id/merge_requests/3/merge") => { lab.mr["state"] = json!("merged"); lab.mr["merge_commit_sha"] = json!("c".repeat(40)); ok(&lab.mr) }
        ("GET", "projects/:id/merge_requests/3/notes") => ok(&json!([])),
        ("POST", "projects/:id/merge_requests/3/notes") => ok(&json!({ "id": 1 })),
        ("GET", "projects/:id/pipelines/9/jobs") => ok(&json!(lab.jobs)),
        ("GET", "projects/:id/pipelines") => ok(&json!([])),
        ("GET", p) if p.starts_with("projects/:id/jobs/") && p.ends_with("/trace") => ok(&json!(lab.trace)),
        _ => GhOut { status: 1, stdout: String::new(), stderr: format!("fake glab: unknown {}", a.join(" ")) },
    }
}
/// A repo whose origin is `remote` (git reaches it in the world's own bare repo), with GitLab answered by the fake.
pub fn lab_world(settings: Value, remote: &str) -> (World, Rc<RefCell<Lab>>) {
    let mut s = json!({ "review.when": "never" });
    for (k, v) in settings.as_object().cloned().unwrap_or_default() { s[k] = v; }
    lab_world_with(Opts { repo: true, origin: true, settings: s, ..Default::default() }, remote)
}
/// The same, from a world's own options (its settings as given, its repo's own file).
pub fn lab_world_with(o: Opts, remote: &str) -> (World, Rc<RefCell<Lab>>) {
    let w = world_with(o);
    let bare = w.base.join("origin.git");
    git(&w.root, &["config", &format!("url.{}.insteadOf", bare.display()), remote]);
    git(&w.root, &["remote", "set-url", "origin", remote]);
    let lab = Rc::new(RefCell::new(Lab::default()));
    { let l = lab.clone(); gitlab::set_fake_glab(Some(Rc::new(move |a: &[String]| answer(&mut l.borrow_mut(), &bare, a)))); }
    (w, lab)
}
/// A job's trace as a recent runner writes it: each line timed and marked, its sections named in it.
pub const TRACE: &str = "2026-10-17T19:05:39.100000Z 00O+section_start:1792256739:step_script\r\x1b[0K\x1b[32;1mExecuting \"step_script\"\x1b[0;m\n2026-10-17T19:05:39.200000Z 01O $ cargo test\n2026-10-17T19:05:59.300000Z 01O test parses_flags ... FAILED\n2026-10-17T19:05:59.400000Z 00O section_end:1792256759:step_script\r\x1b[0K\n2026-10-17T19:06:00.000000Z 00O \x1b[31;1mERROR: Job failed: exit code 1\x1b[0;m\n";
pub fn job(name: &str, status: &str) -> Value { json!({ "id": 42, "name": name, "status": status, "allow_failure": false, "web_url": "https://gitlab.com/o/r/-/jobs/42" }) }
pub fn lab_calls(lab: &Rc<RefCell<Lab>>) -> String { lab.borrow().log.join("\n") }

