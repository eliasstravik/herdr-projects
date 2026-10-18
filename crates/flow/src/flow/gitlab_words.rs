//! What the board says of a pull request on GitHub, said as GitLab says it on a task whose repo is on GitLab: merge
//! request, pipeline, glab. Applied where the board writes on such a task (runner.rs: `Guard::set`, the merge
//! requests' watch) and where it names one (a row, the review page). docs/gitlab.md.
use crate::json::*;

/// A merge request's link (GitLab's `/-/merge_requests/<n>`), told apart without asking anything.
pub fn is_mr(url: &str) -> bool { url.contains("/-/merge_requests/") }
/// What a pull request's link is called: a merge request on GitLab.
pub fn pr_word(url: &str) -> &'static str { if is_mr(url) { "merge request" } else { "pull request" } }
/// The task's repo is on GitLab: its merge request's link says so, else its remote (gitlab.rs).
pub fn on_gitlab(t: &Obj) -> bool { if t.b("pr") { is_mr(t.s("pr")) } else { t.b("id") && super::gitlab::is_gitlab(&crate::store::repos::folder_of(t.s("id"))) } }
/// GitHub's words in a text, as GitLab says them. Longest first: a command before the word it holds.
pub fn say(s: &str) -> String {
    let table: [(&regex::Regex, &str); 15] = [
        (crate::re!(r"gh pr checks(?: [0-9]+)?"), "glab ci status"),
        (crate::re!(r"gh pr view --comments"), "glab mr view --comments"),
        (crate::re!(r"gh pr (view|merge|create|reopen)\b"), "glab mr $1"),
        (crate::re!(r"gh pr edit (\S+) --base"), "glab mr update $1 --target-branch"),
        (crate::re!(r"gh auth status"), "glab auth status"),
        (crate::re!(r"\bgh is logged in"), "glab is logged in"),
        (crate::re!(r"GitHub's checks"), "GitLab's pipeline checks"),
        (crate::re!(r"GitHub checks"), "GitLab pipeline checks"),
        (crate::re!(r"\bGitHub\b"), "GitLab"),
        (crate::re!(r"\bpull request #([0-9]+)"), "merge request !$1"),
        (crate::re!(r"\bmerged in #([0-9]+)"), "merged in !$1"),
        (crate::re!(r"\bpull requests\b"), "merge requests"),
        (crate::re!(r"\bpull request\b"), "merge request"),
        (crate::re!(r"\bPull requests\b"), "Merge requests"),
        (crate::re!(r"\bPull request\b"), "Merge request"),
    ];
    table.iter().fold(s.to_string(), |s, (re, to)| re.replace_all(&s, *to).into_owned())
}
/// `s` as the task's repo says it.
pub fn for_task(t: &Obj, s: &str) -> String { if on_gitlab(t) { say(s) } else { s.to_string() } }
/// What a task says of itself (its line, its failure, its pull request's blocker), in GitLab's words. True: it changed.
pub fn reword(t: &mut Obj) -> bool {
    let mut changed = false;
    let mut fix = |v: &mut Value| if let Some(s) = v.as_str() { let n = say(s); if n != s { *v = json!(n); changed = true; } };
    if let Some(v) = t.get_mut("msg") { fix(v); }
    for k in ["failure", "prFailure"] {
        if let Some(Value::Object(o)) = t.get_mut(k) { for f in ["title", "detail", "next", "why"] { if let Some(v) = o.get_mut(f) { fix(v); } } }
    }
    changed
}
