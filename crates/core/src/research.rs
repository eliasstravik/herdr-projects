//! Reports and research-only tasks: a report never goes into the repo, and a task marked research only changes
//! nothing in it. A report is written into the task's own folder among the board's reports (docs/home-folder.md),
//! outside every repo; the board gives it a private link. A research-only task (the setting `research.only`) has that
//! report as its whole result: the finish line pushes, opens and merges nothing for it, and a change in its worktree or
//! on its branch goes back to its agent.
use crate::clock::now;
use crate::err::Res;
use crate::json::*;
use crate::store::files::{artifacts_dir, reports_dir};
pub use crate::store::old_research::RESEARCH;
use crate::store::repos::{branch_of, cwd_of, git_out};
use crate::store::settings::lookup::setting;
use crate::store::tasks::Task;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Is this task research only? A row of the board says it (as the machine that owns the task worked it out); a task
/// whose agent was started is what it was started as (`researchRun`); one not started yet is what the setting says now.
/// A chat-only task is not: chat wins (chat_only.rs).
pub fn is_research(t: &Task) -> bool {
    if let Some(b) = t.at("research").as_bool() { return b; }
    if t.contains_key("state") || t.b("host") { return false; }
    if crate::chat_only::is_chat(t) { return false; }
    if let Some(b) = t.at("researchRun").as_bool() { if !t.b("researchOpen") { return b; } }
    setting(RESEARCH, Some(t)) == json!(true)
}
/// The same answer for every task of one list: tasks of one repo that say nothing of their own share one lookup.
pub fn is_research_among(t: &Task, seen: &mut BTreeMap<String, bool>) -> bool {
    let asks = !t.at("research").is_boolean() && (!t.at("researchRun").is_boolean() || t.b("researchOpen"));
    if !asks || t.at("settings").as_object().map(|s| s.contains_key(RESEARCH) || s.contains_key(crate::chat_only::CHAT)).unwrap_or(false) || t.b("carried") || t.b("repoFile") { return is_research(t); }
    let repo = if t.s("repo").is_empty() { t.s("id").split('/').next().unwrap_or("").to_string() } else { t.s("repo").to_string() };
    *seen.entry(repo).or_insert_with(|| is_research(t))
}
/// A started task whose setting now says something else than it runs as: said beside the setting, wherever it shows.
pub fn later_words(t: &Task, value: &Value) -> String {
    match t.at("researchRun").as_bool() {
        Some(run) if !t.b("researchOpen") && run != (*value == json!(true)) => format!("counts from its next start: this run is {}", if run { "research only" } else { "not research only" }),
        _ => String::new(),
    }
}
pub const REPORT: &str = "report";
pub fn report_folder(t: &Task) -> PathBuf { let mut p = reports_dir(); for part in t.s("id").split('/') { p.push(part); } p }
/// The page a report opens on: the first of these that is there, else its first file.
const FIRST: [&str; 4] = ["index.html", "report.html", "report.md", "README.md"];

// ---- what agents are told ----------------------------------------------------------------------------------------------
pub fn report_rule(t: &Task, base: &str) -> String {
    format!("A report, a review or a research result (anything you write for the human to read that is not part of the product) never goes into the repo: do not commit one, in no folder. Save it in {}/ (this task's own folder among the board's reports, outside every repo), as index.html (one page that stands by itself) or report.md. {} Never give the folder's path as the report's address: a path only says where the file is, not something the human can open from another machine, and the board writes the published link itself where it has one. Inside the report, link to its own files by name (report.md, a picture beside it): the human reads it from that folder.", report_folder(t).display(), report_place(base))
}
/// The address of a report, as an agent is told it: the page server's own base for this task (the same one the board
/// builds the report's link on) when it serves one, else that nothing publishes it. The host and port are the
/// instance's own: nothing here works them out from the local path.
pub fn report_place(base: &str) -> String {
    if base.is_empty() {
        "No page server publishes the report on this machine: give the file's path and say it is the file's path, not a link; the board says the same in the closing message.".into()
    } else {
        format!("This task's report folder is served at {base}/ (its host and port are this instance's own). The file you save there opens at that address with its own name: give that exact file as a Markdown link, for example [the report]({base}/report.md) for report.md or [the report]({base}/index.html) for index.html, in your closing note and in your last message to the human; the board puts the same published link in the closing message. The link is there when the task finishes or waits for the human. The folder on its own is not a report: it opens nothing.")
    }
}
/// A short form of `report_place` for every prompt the board sends, so an agent already running under older rules is
/// told the exact published file link the next time it hears from the board. Empty base: no link is invented.
pub fn report_hint(base: &str) -> String {
    if base.is_empty() {
        "If you write a report for the human, give its local path and say that no published link exists on this machine.".into()
    } else {
        format!("If you write a report for the human, link the file you saved by its own name under {base}/, for example [the report]({base}/report.md); never the bare folder and never a local path.")
    }
}
pub fn research_rule(t: &Task) -> String {
    let look = if t.b("worktree") { format!("worktree and at branch {}", t.s("branch")) } else { "folder".into() };
    format!("This task is research only: change nothing in the repo. Add, edit and delete no file, and commit nothing. Your result is a report, saved as said below, and the work that should follow from it is a short list in that report (\"Also noticed\"): nothing else is done with it, and you add no task for it. When you finish, the board looks at this {look}: if anything changed, committed or not, nothing is pushed or merged and it comes back to you. Saving the report finishes nothing: you may save it early and add to it while you still wait for the human's answers. The task is finished when you stop without saying that you wait (the report command below).")
}
pub fn research_tree(t: &Task) -> String {
    format!("You are in your own git worktree on branch {}; the human's checkout is elsewhere and untouched. Commit nothing on this branch, and do not push, merge or switch branches. When you finish, the board keeps your report and opens no pull request.", t.s("branch"))
}
pub fn review_rule(t: &Task) -> String {
    format!("This task is research only: nothing in the repository may change, and its whole result is a report, in {}/ (outside the repository; the human gets a private link to it). You are in the folder where the work was done, with all your tools. Read the report whole and check it against what was asked: does it answer all of it, correctly? Check what it states against the code, the sources and the commands it speaks of: a claim that is false, or that nothing shows, is a finding. No report there is a critical finding. The agent was told to change nothing in the repository: look for yourself (`git status`, `git log`, other branches, anything pushed) and report any file it added, changed or deleted, and any commit, as a critical finding. Ignore taste and style nits.", report_folder(t).display())
}

// ---- what a research-only task changed ---------------------------------------------------------------------------------
/// The paths git calls changed in a folder (new files too), each with what it holds: the file's content and mode (a
/// hash; for a very large file its size and when it was written) and what is staged of it.
fn dirty_of(dir: &Path) -> Res<BTreeMap<String, String>> {
    let mut staged: BTreeMap<String, String> = BTreeMap::new();
    let raw = git_out(dir, &["diff", "--cached", "--raw", "--no-abbrev", "-z", "HEAD"])?;
    let raw: Vec<&str> = raw.split('\0').collect();
    let mut n = 0;
    while n < raw.len() {
        if raw[n].starts_with(':') {
            let entry = raw[n];
            let letter = entry.split(' ').nth(4).unwrap_or("");
            let count = if letter.starts_with(['R', 'C']) { 2 } else { 1 };
            for _ in 0..count { n += 1; if let Some(rel) = raw.get(n).filter(|r| !r.is_empty()) { staged.insert(rel.to_string(), entry.to_string()); } }
        }
        n += 1;
    }
    let mut names: BTreeSet<String> = staged.keys().cloned().collect();
    let st = git_out(dir, &["status", "--porcelain", "--untracked-files=all", "-z"])?;
    let entries: Vec<&str> = st.split('\0').collect();
    let mut n = 0;
    while n < entries.len() {
        let e = entries[n];
        // Two status letters, a blank, the path (the first entry may have lost a leading blank).
        // Two letters first: "M  a.txt" (staged only) is the letters "M " and the path "a.txt", not " a.txt".
        let split = [2, 1].into_iter().find(|&i| e.len() > i + 1 && e.as_bytes()[i] == b' ' && e.is_char_boundary(i) && e[..i].chars().all(|c| " MADRCUT?!".contains(c)));
        if let Some(i) = split {
            let letters = &e[..i];
            names.insert(e[i + 1..].to_string());
            if letters.contains(['R', 'C']) && entries.get(n + 1).map(|x| !x.is_empty()).unwrap_or(false) { n += 1; names.insert(entries[n].to_string()); }
        }
        n += 1;
    }
    let mut hidden: BTreeMap<String, char> = BTreeMap::new();
    for l in git_out(dir, &["ls-files", "-v", "-z"])?.split('\0') {
        let mut cs = l.chars();
        if let (Some(c), Some(' ')) = (cs.next(), cs.next()) { if c.is_ascii_lowercase() || c == 'S' { hidden.insert(l[2..].to_string(), c); names.insert(l[2..].to_string()); } }
    }
    let mut out = BTreeMap::new();
    for rel in names {
        use std::os::unix::fs::MetadataExt;
        let f = dir.join(&rel);
        let file = match fs::symlink_metadata(&f) {
            Ok(s) => format!("{:o}:{}", s.mode(), if s.is_file() && s.len() < 8_000_000 { fs::read(&f).map(|b| sha1_smol::Sha1::from(b).digest().to_string()).unwrap_or_default() } else { format!("{}:{}", s.len(), s.mtime() * 1000 + s.mtime_nsec() / 1_000_000) }),
            Err(_) => "gone".into(),
        };
        let line = format!("{file} {}{}", staged.get(&rel).map(|s| s.as_str()).unwrap_or("-"), hidden.get(&rel).map(|c| format!(" hidden:{c}")).unwrap_or_default());
        out.insert(rel, line);
    }
    Ok(out)
}
fn commits_since(dir: &Path, from: &str) -> i64 { git_out(dir, &["rev-list", "--count", &format!("{from}..HEAD")]).ok().and_then(|s| s.parse().ok()).unwrap_or(0) }
fn commit_words(n: i64) -> String { format!("({n} {} added)", if n == 1 { "commit" } else { "commits" }) }
/// The checkout as it stands when the agent starts: its commit, and every changed file. None with nothing to compare.
pub fn start_mark(dir: &Path) -> Option<Value> {
    let head = git_out(dir, &["rev-parse", "HEAD"]).ok()?;
    let dirty = dirty_of(dir).ok()?;
    Some(json!({ "head": head, "dirty": dirty }))
}
/// Everything that changed. None: it cannot be told (no mark was taken).
pub fn changes_of(t: &Task, dir: &Path) -> Option<Vec<String>> {
    let mut seen: Vec<String> = vec![];
    let add = |s: String, seen: &mut Vec<String>| if !seen.contains(&s) { seen.push(s); };
    if !t.b("worktree") {
        let was = t.at("startMark");
        if !was.is_object() { return None; }
        let now = dirty_of(dir).ok()?;
        let before: BTreeMap<String, String> = was.o("dirty").map(|o| o.iter().map(|(k, v)| (k.clone(), text(v))).collect()).unwrap_or_default();
        let keys: BTreeSet<&String> = now.keys().chain(before.keys()).collect();
        for rel in keys { if now.get(rel) != before.get(rel) { add(rel.clone(), &mut seen); } }
        let n = commits_since(dir, was.s("head"));
        if n > 0 { add(commit_words(n), &mut seen); }
        else if git_out(dir, &["rev-parse", "HEAD"]).map(|h| h != was.s("head")).unwrap_or(false) { add("(the checkout is on another commit than when the agent started)".into(), &mut seen); }
        return Some(seen);
    }
    if let Ok(d) = dirty_of(dir) { for rel in d.keys() { add(rel.clone(), &mut seen); } }
    let base = if t.b("baseSha") { t.s("baseSha") } else { t.s("from") };
    for from in [t.s("syncedTo"), base].into_iter().filter(|s| !s.is_empty()) {
        let Ok(out) = git_out(dir, &["diff", "--name-only", &format!("{from}...HEAD")]) else { continue };
        for l in out.lines() { if !l.trim().is_empty() { add(l.trim().to_string(), &mut seen); } }
        let n = commits_since(dir, from);
        if n > 0 { add(commit_words(n), &mut seen); }
        break;
    }
    let on = branch_of(dir);
    if t.b("branch") && on != t.s("branch") { add(format!("(the worktree is on {}, not on {})", if on.is_empty() { "no branch" } else { &on }, t.s("branch")), &mut seen); }
    Some(seen)
}
/// The moment an agent starts: what the setting says now is what this run is, kept on the task until the next start.
pub fn starts_as(t: &mut Task) -> Res<()> {
    let only = setting(RESEARCH, Some(t)) == json!(true) && setting(crate::chat_only::CHAT, Some(t)) != json!(true);
    let why = turn_refusal(t, only);
    if !why.is_empty() { crate::fail!("{why} To start it as it was: herdr-projects set {RESEARCH} false --task {}", t.s("id")); }
    if let Some(run) = t.at("researchRun").as_bool() { if run != only && t.b("startedAgentAt") { t.put("researchTurned", true); } }
    t.put("researchRun", only);
    crate::chat_only::starts_as(t);
    t.del("researchOpen");
    Ok(())
}
/// Turning research only on for a task started as an ordinary one: the reason it is refused, or "".
pub fn turn_refusal(t: &Task, to: bool) -> String {
    if !to || t.at("researchRun") != &json!(false) || setting(crate::chat_only::CHAT, Some(t)) == json!(true) { return String::new(); }
    let id = t.s("id");
    let instead = format!("Leave {id} as it is and add a new task as research only, or");
    if t.b("pr") && !t.b("prMerged") && t.s("prState") != "closed" {
        return format!("{RESEARCH} cannot be turned on for {id}: its pull request is open ({}), and a research-only task opens none. {instead} close the pull request first.", t.s("pr"));
    }
    let dir = cwd_of(t);
    let list = if !t.b("worktree") || dir == Path::new(t.s("worktree")) { changes_of(t, &dir) } else { None };
    if let Some(list) = list.filter(|l| !l.is_empty()) {
        return format!("{RESEARCH} cannot be turned on for {id}: its agent has already changed the repo ({}: {}{}), and a research-only task changes nothing in it. {instead} undo the changes first.",
            changed_words(&list), list.iter().take(3).cloned().collect::<Vec<_>>().join(", "), if list.len() > 3 { ", …" } else { "" });
    }
    String::new()
}
pub fn changed_words(list: &[String]) -> String { format!("{} {} in the repo", list.len(), if list.len() == 1 { "change" } else { "changes" }) }
/// What the agent is told when its research-only task changed the repo, and what the task shows if it does so again.
pub fn changed_failure(t: &Task, tree: &str, list: &[String], rounds: Option<i64>) -> Obj {
    let mut shown = list.iter().take(30).map(|f| format!("- {f}")).collect::<Vec<_>>().join("\n");
    if list.len() > 30 { shown.push_str(&format!("\n- … and {} more", list.len() - 30)); }
    let gave_up = rounds.map(|r| format!(" after {r} {}", if r == 1 { "round" } else { "rounds" })).unwrap_or_default();
    let id = t.s("id");
    let folder = report_folder(t).display().to_string();
    let start: String = [t.s("syncedTo"), t.s("baseSha"), t.s("from")].into_iter().find(|s| !s.is_empty()).unwrap_or("").chars().take(12).collect();
    let next = if t.b("worktree") {
        format!("Move the result out of the repo: save the report in {folder}/ (outside the repo), then undo every change, the commits too, so that the branch is where it started ({start}) with no commit of yours on it and `git status` is clean. If the change is wanted after all, the human turns research only off for this task (herdr-projects set research.only false --task {id}) and delegates it again.")
    } else {
        format!("This is the human's own checkout, and it may hold changes of theirs: undo only what you changed yourself, file by file, and never reset or clean the checkout. Move the result out of the repo first: save the report in {folder}/ (outside the repo). Take back a commit of yours without losing anything of the human's. If a listed change is not yours, or the change is wanted after all, say so and stop: the human turns research only off for this task (herdr-projects set research.only false --task {id}) and delegates it again, or retries.")
    };
    let place = if t.b("worktree") { format!("Worktree: {tree}\nBranch: {}", t.s("branch")) } else { format!("Folder (your own checkout): {tree}") };
    obj(json!({
        "stage": "research", "auth": false, "agentCanFix": true, "title": format!("Research only, but the repo was changed{gave_up}"),
        "msg": format!("Research only, but its agent changed the repo{} ({}): nothing was pushed, opened or merged", if gave_up.is_empty() { String::new() } else { format!(" again{gave_up}") }, changed_words(list)),
        "detail": format!("This task is research only: it may change nothing in the repo. Nothing was committed, pushed, opened as a pull request or merged.\n\n{place}\n\nChanged{}:\n{shown}", if t.b("worktree") { "" } else { " since the agent started" }),
        "next": next,
    }))
}

// ---- the report --------------------------------------------------------------------------------------------------------
fn files_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir).map(|rd| rd.filter_map(|e| e.ok()).filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| !n.starts_with('.')).collect()).unwrap_or_default();
    v.sort();
    v
}
pub fn report_store(id: &str) -> PathBuf { artifacts_dir(id).join(REPORT) }
/// A report as it reads in a message to a reader (the human, another task's agent): the link the board published, as a
/// Markdown link a click opens, else that nothing publishes it with the file's path as the second thing said. The link
/// is used whole, as `keep_report` kept it (this instance's own host and port), and never worked out from the path.
/// "" when the task has no report at all.
pub fn report_words(file: &str, link: &str) -> String {
    if link.is_empty() { return String::new(); }
    let file = if file.is_empty() { "report" } else { file };
    if link.starts_with("http://") || link.starts_with("https://") { format!("[{file}]({link})") }
    else { format!("{file} (no published link; the file is at {link})") }
}
/// The report the agent wrote, and its link: what the task remembers ({ file, link, at }), or None. The page server only
/// serves a task's own folder in the board's data, so that folder gets a link to the report folder. `base` answers
/// where the task's report folder is served from ("" when nothing serves it: then the link is the file).
pub fn keep_report(t: &Task, base: impl FnOnce() -> String) -> Option<Value> {
    let from = report_folder(t);
    let names = files_in(&from);
    let file = FIRST.iter().find(|n| names.iter().any(|x| x == *n)).map(|s| s.to_string()).or_else(|| names.first().cloned())?;
    let to = report_store(t.s("id"));
    let mut root = String::new();
    let there = fs::read_link(&to).ok();
    let ok = if there.as_deref() != Some(from.as_path()) {
        let _ = fs::remove_dir_all(&to); let _ = fs::remove_file(&to);
        to.parent().map(|p| fs::create_dir_all(p).is_ok()).unwrap_or(false) && std::os::unix::fs::symlink(&from, &to).is_ok()
    } else { true };
    if ok { root = base(); }
    let link = if root.is_empty() { from.join(&file).display().to_string() } else { format!("{root}/{}", crate::util::url_encode(&file)) };
    Some(json!({ "file": file, "link": link, "at": now() }))
}
/// A closing note or message as the board hands it on: a bare path to this task's own report file (or its folder)
/// becomes the published link the board kept for that very file. Nothing is guessed: only those exact paths are
/// replaced, and only when the board has a published link. An agent already started under older rules is covered here.
pub fn report_linked(text: &str, t: &Task) -> String {
    let r = t.at("report");
    let link = r.s("link");
    if text.is_empty() || !(link.starts_with("http://") || link.starts_with("https://")) { return text.to_string(); }
    let file = if r.b("file") { r.s("file") } else { "report" };
    let shown = format!("[{file}]({link})");
    let folder = report_folder(t).display().to_string();
    let mut out = text.replace(&format!("{folder}/{file}"), &shown);
    for f in [format!("{folder}/"), folder] { out = out.replace(&f, &shown); }
    // An agent started under the old rules linked the published folder: it opens nothing. The board's kept link names
    // the file itself, so it is used whole. Only when the text does not already hold it, so the link is never doubled.
    if !out.contains(&link) { if let Some((base, _)) = link.rsplit_once('/') { out = out.replace(&format!("{base}/"), &link); if !out.contains(&link) { out = out.replace(base, &link); } } }
    out
}
/// A message to the human as the board hands it on: the report's published link in it, as a Markdown link, once. A bare
/// path is replaced by `report_linked`; if the message does not name the report at all, the link is added as a short
/// "Report:" line. Where nothing publishes it, the line says so and the path comes second. Already-finished agents
/// under older rules are covered here: the board writes the closing surfaces, not their rules.
pub fn report_in_message(text: &str, t: &Task) -> String {
    let text = report_linked(text, t);
    let rep = t.at("report");
    let link = rep.s("link");
    let words = report_words(rep.s("file"), link);
    if words.is_empty() { return text; }
    // A link already in Markdown (or an autolink) is kept as it is; a bare URL becomes the Markdown link; no URL at
    // all gets the short line.
    if text.contains(&format!("]({link})")) || text.contains(&format!("<{link}>")) { return text; }
    if text.contains(link) { return text.replace(link, &words); }
    format!("{}{}Report: {}.", text, if text.is_empty() || text.ends_with('\n') { "" } else { " " }, words)
}
