//! A pull request on Azure DevOps, through `az repos pr`: opened as a draft, found for a branch, read in the shape gh
//! gives (`state` OPEN/MERGED/CLOSED, `headRefOid`, `baseRefName`, `mergeCommit`, `reviewDecision`, `author`,
//! `comments`, `statusCheckRollup`), marked ready, completed by merge strategy, and commented on.
use super::cli::{invoke, json_of, run, said};
use super::policies;
use super::url::Azure;
use crate::flow::git::GhOut;
use crate::json::*;
use std::path::Path;

/// A pull request's link is one of Azure DevOps': `…/_git/<repo>/pullrequest/<id>`.
pub fn is_azure(pr: &str) -> bool { pr.contains("/pullrequest/") }
/// Its number, from its link (or the number itself).
pub fn id_of(pr: &str) -> String { pr.split("/pullrequest/").nth(1).map(|r| r.chars().take_while(|c| c.is_ascii_digit()).collect()).filter(|s: &String| !s.is_empty()).unwrap_or_else(|| pr.trim_start_matches(['!', '#']).to_string()) }
fn branch(r: &str) -> String { r.strip_prefix("refs/heads/").unwrap_or(r).to_string() }
/// Azure DevOps' status of a pull request, as gh words a state.
pub fn state_of(status: &str) -> &'static str { match status.to_lowercase().as_str() { "completed" => "MERGED", "abandoned" => "CLOSED", _ => "OPEN" } }
/// What its reviewers' votes say, as gh's `reviewDecision`: rejected or waiting for the author is a change asked for;
/// `required`: a blocking policy of reviewers (their number, required reviewers, resolved comments) is not met yet.
pub fn decision(reviewers: &[Value], required: bool) -> &'static str {
    let votes: Vec<i64> = reviewers.iter().map(|r| r.i("vote")).collect();
    if votes.iter().any(|v| *v < 0) { return "CHANGES_REQUESTED"; }
    if required || reviewers.iter().any(|r| r.b("isRequired") && r.i("vote") <= 0) { return "REVIEW_REQUIRED"; }
    if votes.iter().any(|v| *v >= 5) { "APPROVED" } else { "" }
}
/// One pull request as az gives it, in gh's shape (without its checks and comments).
pub fn shaped(a: &Azure, p: &Value) -> Value {
    let id = p.t("pullRequestId");
    let state = state_of(p.s("status"));
    let merge = p.at("lastMergeCommit").t("commitId");
    json!({
        "number": id, "url": a.pr_url(&id), "state": state, "title": p.at("title"), "isDraft": p.b("isDraft"),
        "baseRefName": branch(p.s("targetRefName")), "headRefName": branch(p.s("sourceRefName")), "headRefOid": p.at("lastMergeSourceCommit").t("commitId"),
        // On an open one, its last merge commit is only Azure DevOps' trial merge.
        "mergeCommit": if state == "MERGED" && !merge.is_empty() { json!({ "oid": merge }) } else { Value::Null },
        "mergeStatus": p.at("mergeStatus"), "author": { "login": p.at("createdBy").at("uniqueName") }, "reviewDecision": decision(p.arr("reviewers"), false),
        "reviewers": p.arr("reviewers").iter().map(|r| json!({ "name": r.at("displayName"), "vote": r.at("vote"), "required": r.b("isRequired") })).collect::<Vec<_>>(),
    })
}
/// Opens the draft pull request of `branch` into `onto` (the repo's default branch when None). Its link is what it
/// prints, as `gh pr create` does; a failure is az's own answer.
pub fn create(a: &Azure, cwd: &Path, branch: &str, onto: Option<&str>, title: &str, body: &str) -> GhOut {
    // Azure DevOps takes at most 4000 characters of description.
    let body: String = if body.chars().count() > 4000 { format!("{}…", body.chars().take(3990).collect::<String>()) } else { body.to_string() };
    let mut args = vec!["repos", "pr", "create", "--project", &a.project, "--repository", &a.repo, "--source-branch", branch, "--title", title, "--description", &body, "--draft", "true"];
    if let Some(b) = onto { args.extend(["--target-branch", b]); }
    let mut r = run(a, cwd, &args, 120000);
    if r.status == 0 {
        let id = serde_json::from_str::<Value>(r.stdout.trim()).map(|v| v.t("pullRequestId")).unwrap_or_default();
        r.stdout = if id.is_empty() { String::new() } else { format!("{}\n", a.pr_url(&id)) };
        if id.is_empty() { r.status = 1; r.stderr = format!("az repos pr create gave no pull request:\n{}", said(&r)); }
    }
    r
}
/// The pull requests of this head branch, newest first, in gh's shape. None when az cannot say.
pub fn prs_of(a: &Azure, cwd: &Path, branch: &str) -> Vec<Value> {
    let Ok(v) = json_of(a, cwd, &["repos", "pr", "list", "--project", &a.project, "--repository", &a.repo, "--source-branch", branch, "--status", "all", "--top", "10"], 60000) else { return vec![] };
    let mut all: Vec<Value> = v.as_array().map(|l| l.iter().map(|p| shaped(a, p)).collect()).unwrap_or_default();
    all.sort_by_key(|p| std::cmp::Reverse(p.t("number").parse::<i64>().unwrap_or(0)));
    all
}
fn show(a: &Azure, cwd: &Path, pr: &str) -> Result<Value, String> { json_of(a, cwd, &["repos", "pr", "show", "--id", &id_of(pr)], 60000) }
/// The comments people wrote on it (not Azure DevOps' own: votes, pushes), as gh's `comments`.
fn comments(a: &Azure, cwd: &Path, pr: &str) -> Vec<Value> {
    let Ok(v) = invoke(a, cwd, "git", "pullRequestThreads", &[("repositoryId", &a.repo), ("pullRequestId", &id_of(pr))], "GET", None) else { return vec![] };
    v.arr("value").iter().flat_map(|th| th.arr("comments").iter().filter(|c| c.s("commentType") != "system" && !c.b("isDeleted")).map(move |c| json!({
        "id": format!("{}/{}", th.t("id"), c.t("id")), "author": { "login": c.at("author").at("uniqueName") }, "body": c.at("content"), "updatedAt": if c.b("lastUpdatedDate") { c.at("lastUpdatedDate").clone() } else { c.at("publishedDate").clone() },
    }))).collect()
}
/// The pull request as `gh pr view --json <fields>` gives it: with its checks when `fields` asks for
/// `statusCheckRollup` (its policies: policies.rs, which also decide `reviewDecision`), and the comments when it asks
/// for `comments`. Err: what az said.
pub fn view(a: &Azure, cwd: &Path, pr: &str, fields: &str) -> Result<Value, String> {
    let p = show(a, cwd, pr)?;
    let mut v = shaped(a, &p);
    let wants = |f: &str| fields.split(',').any(|x| x == f);
    if wants("statusCheckRollup") || wants("reviewDecision") {
        // Merged or abandoned: its policies are not asked for.
        let (checks, required) = if v.s("state") == "OPEN" { policies::read(a, cwd, pr)? } else { (vec![], false) };
        v["statusCheckRollup"] = json!(checks);
        v["reviewDecision"] = json!(decision(p.arr("reviewers"), required));
    }
    v["comments"] = json!(if wants("comments") && v.s("state") == "OPEN" { comments(a, cwd, pr) } else { vec![] });
    v["reviews"] = json!([]);
    Ok(v)
}
/// `view` as gh prints it: the JSON on stdout, or az's failure.
pub fn view_out(a: &Azure, cwd: &Path, pr: &str, fields: &str) -> GhOut {
    match view(a, cwd, pr, fields) { Ok(v) => GhOut { status: 0, stdout: v.to_string(), stderr: String::new() }, Err(e) => GhOut { status: 1, stdout: String::new(), stderr: e } }
}
/// A draft no more, so that it can be completed.
pub fn ready(a: &Azure, cwd: &Path, pr: &str) -> GhOut { run(a, cwd, &["repos", "pr", "update", "--id", &id_of(pr), "--draft", "false"], 60000) }
/// How `merge.method` is said to Azure DevOps.
pub fn strategy(how: &str) -> &'static str { match how { "merge" => "noFastForward", "rebase" => "rebase", _ => "squash" } }
/// Completes it at `head` only (Azure DevOps refuses it once the branch has moved on), by `merge.method`, its source
/// branch deleted. `az repos pr update` cannot say a strategy other than squash: this is its REST call.
pub fn complete(a: &Azure, cwd: &Path, pr: &str, head: &str, how: &str, message: &str) -> GhOut {
    let mut body = json!({ "status": "completed", "completionOptions": { "mergeStrategy": strategy(how), "deleteSourceBranch": true, "mergeCommitMessage": message } });
    if !head.is_empty() { body["lastMergeSourceCommit"] = json!({ "commitId": head }); }
    match invoke(a, cwd, "git", "pullRequests", &[("repositoryId", &a.repo), ("pullRequestId", &id_of(pr))], "PATCH", Some(&body)) {
        Ok(v) => GhOut { status: 0, stdout: v.to_string(), stderr: String::new() },
        Err(e) => GhOut { status: 1, stdout: String::new(), stderr: e },
    }
}
/// A comment of the board's own (the review), as a thread that is closed at once: it never holds the merge up where
/// comments must be resolved.
pub fn comment(a: &Azure, cwd: &Path, pr: &str, body: &str) -> GhOut {
    let thread = json!({ "comments": [{ "parentCommentId": 0, "content": body, "commentType": "text" }], "status": "closed" });
    match invoke(a, cwd, "git", "pullRequestThreads", &[("repositoryId", &a.repo), ("pullRequestId", &id_of(pr))], "POST", Some(&thread)) {
        Ok(_) => GhOut { status: 0, stdout: String::new(), stderr: String::new() },
        Err(e) => GhOut { status: 1, stdout: String::new(), stderr: e },
    }
}
