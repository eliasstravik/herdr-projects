//! GitLab, on gitlab.com and on a host of your own (`board.gitlabHosts`): the repo's remote says which, and the board then
//! opens a merge request, waits for its pipeline and merges it through `glab api`, never gh. A fake glab answers here.
use crate::common;
use common::*;
use common::glab::*;
use serde_json::json;
use std::path::Path;
use sw::actions::finish_line;
use sw::flow::gitlab;
use sw::housekeeping::poll_prs;
use sw::json::*;

#[test]
fn the_remote_says_whether_a_repo_is_on_gitlab_gitlab_com_or_a_host_set_in_gitlab_hosts() {
    for (url, host) in [("git@gitlab.com:o/r.git", "gitlab.com"), ("https://gitlab.com/o/r.git", "gitlab.com"), ("ssh://git@git.corp.example:2222/o/r.git", "git.corp.example"),
        ("https://user:tok@git.corp.example:8443/g/sub/r", "git.corp.example"), ("/tmp/origin.git", "")] {
        assert_eq!(gitlab::host_of(url), host, "{url}");
    }
    let (w, _) = lab_world(json!({}), "git@gitlab.com:o/r.git");
    assert!(gitlab::is_gitlab(&w.root));
    git(&w.root, &["remote", "set-url", "origin", "git@github.com:o/r.git"]);
    assert!(!gitlab::is_gitlab(&w.root), "GitHub stays GitHub");
    git(&w.root, &["remote", "set-url", "origin", "git@git.corp.example:o/r.git"]);
    assert!(!gitlab::is_gitlab(&w.root), "a host of your own only once it is named");
    sw::store::settings::change::set_value("board.gitlabHosts", &json!("https://git.corp.example/, other.example"), &sw::store::settings::lookup::Scope::you()).unwrap();
    assert!(gitlab::is_gitlab(&w.root));
    assert_eq!(sw::flow::git::pr_number("https://git.corp.example/g/r/-/merge_requests/12"), "12");
}

#[test]
fn on_gitlab_the_board_opens_a_merge_request_waits_for_its_pipeline_and_merges_the_reviewed_commit() {
    let (w, lab) = lab_world(json!({}), "git@gitlab.com:o/r.git");
    let id = to_approval(&w, "On GitLab");
    let t = task(&id);
    assert_eq!(t.s("pr"), MR, "{}", lab_calls(&lab));
    assert!(lab_calls(&lab).contains(&format!("-f source_branch={}", t.s("branch"))) && lab_calls(&lab).contains("-f title=Draft: On GitLab"), "{}", lab_calls(&lab));
    assert_eq!(gh_log(&w), "", "gh is never asked");
    let reviewed = git(Path::new(t.s("worktree")), &["rev-parse", "HEAD"]);
    // its pipeline still running: the approval is remembered, and the merge waits for it
    lab.borrow_mut().jobs = vec![job("test", "running")];
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "{}", t.at("failure")); assert!(t.b("autoMerge"));
    assert!(!lab_calls(&lab).contains("/merge "), "{}", lab_calls(&lab));
    assert!(lab_calls(&lab).contains("-X PUT projects/:id/merge_requests/3 -f title=On GitLab"), "marked ready: {}", lab_calls(&lab));
    // passed: it merges, at the reviewed commit only
    lab.borrow_mut().jobs = vec![job("test", "success")];
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}\n{}", t.at("failure"), lab_calls(&lab));
    assert!(lab_calls(&lab).contains(&format!("-X PUT projects/:id/merge_requests/3/merge -f sha={reviewed} -F squash=true")), "{}", lab_calls(&lab));
}

#[test]
fn a_failed_gitlab_job_goes_back_with_the_end_of_its_log_and_merge_method_merge_does_not_squash() {
    let (w, lab) = lab_world(json!({ "board.gitlabHosts": "git.corp.example", "review.rounds": 0, "merge.method": "merge" }), "git@git.corp.example:o/r.git");
    let id = to_approval(&w, "Fails on GitLab");
    { let mut l = lab.borrow_mut(); l.jobs = vec![job("test", "failed")]; l.trace = TRACE.into(); }
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", lab_calls(&lab));
    let detail = t.at("prFailure").t("detail");
    assert!(detail.contains("\n$ cargo test\ntest parses_flags ... FAILED\nERROR: Job failed: exit code 1"), "the runner's marks are not the log: {detail}");
    assert!(!detail.contains("section_") && !detail.contains("00O"), "{detail}");
    finish_line::approve(&id, None, true).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(lab_calls(&lab).contains("merge_requests/3/merge -f sha=") && lab_calls(&lab).contains("-F squash=false"), "{}", lab_calls(&lab));
}

#[test]
fn a_keeper_started_without_glab_login_takes_it_from_the_login_shell_and_a_process_with_its_own_keeps_it() {
    let out = "PATH=/x\nGITLAB_TOKEN=t0k\nGITLAB_HOST=git.corp.example\nGH_TOKEN=gh\n";
    let pair = |k: &str, v: &str| (k.to_string(), v.to_string());
    assert_eq!(sw::login_env::glab_login(out, |_| false), vec![pair("GITLAB_TOKEN", "t0k"), pair("GITLAB_HOST", "git.corp.example")]);
    assert_eq!(sw::login_env::glab_login(out, |k| k == "GH_TOKEN").len(), 2, "gh's login of its own is not glab's");
    assert!(sw::login_env::glab_login(out, |k| k == "GITLAB_TOKEN").is_empty(), "a login of its own is kept");
    assert!(sw::store::failures::is_auth_failure("glab: 401 Unauthorized (HTTP 401)"), "glab's own words for a login missing");
    assert_eq!(strings(sw::login_env::remembered(out, "").at("glab")), vec!["GITLAB_TOKEN", "GITLAB_HOST"], "only the names are kept");
}
