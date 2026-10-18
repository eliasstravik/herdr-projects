//! A chat with the agent of a task whose work was reviewed: no code changed, the task goes back to where it was with
//! the same review and nothing runs; code changed, one review once you are done (`review.afterChat` quiet, or you
//! left the chat), however many messages; a review under way goes on unless the code changes; an approval given is
//! withdrawn when the code changes, and nothing unreviewed merges.
use crate::common;
use common::*;
use serde_json::json;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use sw::herdr::cli::{Fake, Ran};
use sw::json::*;

fn tell(id: &str, words: &str) -> String { sw::actions::talk::reply(id, words, "you").unwrap_or_else(|e| panic!("tell: {}", e.msg)).msg }
fn reviews_started(w: &World) -> usize { calls(w).iter().filter(|c| regex::Regex::new(r"^agent start \S+-r ").unwrap().is_match(c)).count() }
fn housekeeping(w: &World) { sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes); }
fn says(id: &str) -> String { sw::shown::row_says::sentence(&row(id)).text() }
/// The board's clock stands still from here: the quiet time passes only when the test moves it.
fn hold_clock() { sw::clock::set(Some(sw::clock::now())); }

#[test]
fn a_chat_that_changes_no_code_keeps_the_stage_and_the_review_and_runs_nothing() {
    let w = world();
    let id = to_approval(&w, "Make f");
    hold_clock();
    let before = task(&id);
    w.script(json!({ "worker": [{ "note": "It writes v1 into f.txt, nothing else." }] }));
    assert_eq!(tell(&id, "What does it do exactly?"), "Sent to the agent");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!((t.at("review").b("stale"), t.i("reviewRound"), t.at("reviewedHead")), (false, before.i("reviewRound"), before.at("reviewedHead")));
    assert_eq!(t.at("review").at("text"), before.at("review").at("text"));
    assert_eq!(t.s("msg"), before.s("msg"));
    assert!(!t.b("chat") && !t.b("requireApproval"), "{}", t.at("chat"));
    assert_eq!(reviews_started(&w), 1, "no second review");
    assert!(says(&id).ends_with("Chatting · no changes, review kept"), "{}", says(&id));
    // the details say it under the review that still stands
    let kept = t.arr("reviews").last().unwrap().arr("kept").to_vec();
    assert!(kept.len() == 1 && kept[0].b("chat"), "{kept:?}");
    assert_eq!(sw::store::reviews::kept_words(&kept[0], 1), "a chat changed no code, review 1 still stands");
    // and nothing waits to run: no quiet time, no review later
    sw::clock::advance(10 * 60_000); housekeeping(&w);
    let t = settle(&id);
    assert_eq!((t.s("stage"), reviews_started(&w)), ("approval".into(), 1));
    // the next message starts a chat of its own: the line goes until it ends
    w.script(json!({ "worker": [{ "status": "working" }] }));
    tell(&id, "And the tests?"); settle(&id);
    assert!(!says(&id).contains("review kept"), "{}", says(&id));
}

#[test]
fn a_chat_that_changes_the_code_is_reviewed_once_after_the_quiet_time() {
    let w = world();
    let id = to_approval(&w, "Make f");
    hold_clock();
    w.script(json!({ "worker": [{ "files": { "f.txt": "v2\n" }, "note": "f.txt says v2 now." }] }));
    tell(&id, "Make it v2");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working"); assert!(t.at("chat").b("held"), "{}", t.at("chat"));
    assert_eq!(row(&id).s("msg"), "Changes made · review starts when you're done");
    assert_eq!(reviews_started(&w), 1, "no review yet");
    // not yet quiet long enough
    sw::clock::advance(59_000); housekeeping(&w); settle(&id);
    assert_eq!((task(&id).s("stage"), reviews_started(&w)), ("working".into(), 1));
    sw::clock::advance(2_000); housekeeping(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!((reviews_started(&w), t.i("reviewRound")), (2, 2));
    assert!(!t.at("review").b("stale") && !t.b("chat"));
    let tree = std::path::PathBuf::from(t.s("worktree"));
    assert_eq!(read(&tree.join("f.txt")), "v2\n"); assert_eq!(t.s("reviewedHead"), git(&tree, &["rev-parse", "HEAD"]));
    assert!(git(&tree, &["status", "--porcelain"]).is_empty(), "the chat's change is committed and reviewed");
}

#[test]
fn several_messages_in_a_row_get_one_review_and_leaving_the_chat_on_the_board_starts_it() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.afterChat": "2m" }), ..Default::default() });
    let id = to_approval(&w, "Make f");
    hold_clock();
    w.script(json!({ "worker": [{ "files": { "f.txt": "v2\n" }, "note": "v2." }, { "files": { "f.txt": "v3\n" }, "note": "v3." }, { "note": "Yes, v3 is final." }] }));
    tell(&id, "Make it v2"); settle(&id);
    sw::clock::advance(90_000); housekeeping(&w); settle(&id);
    tell(&id, "No, v3"); settle(&id);
    sw::clock::advance(90_000); housekeeping(&w); settle(&id);
    tell(&id, "Is that final?"); settle(&id);
    sw::clock::advance(90_000); housekeeping(&w);
    let t = settle(&id);
    assert_eq!((t.s("stage"), reviews_started(&w)), ("working".into(), 1), "each message starts the quiet time again");
    assert_eq!(row(&id).s("msg"), "Changes made · review starts when you're done");
    // you select another task on the board: you are done with this chat
    let other = w.add("Something else");
    let mut s = common::board::open(&w.root, 34);
    s.s.sel = other.clone(); s.load();
    assert!(task(&id).at("chat").b("left"), "{}", task(&id).at("chat"));
    housekeeping(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!((reviews_started(&w), t.i("reviewRound")), (2, 2), "one review for all three messages");
    assert_eq!(read(&std::path::PathBuf::from(t.s("worktree")).join("f.txt")), "v3\n");
}

/// The world's fake herdr, with a look at every call first, so the test can act in the middle of a review.
struct Hooked { inner: Rc<herdr::FakeHerdr>, on: RefCell<Option<Box<dyn Fn(&[String])>>>, busy: Cell<bool> }
impl Fake for Hooked {
    fn run(&self, args: &[String], input: &str) -> Ran {
        if !self.busy.get() { if let Some(f) = self.on.borrow().as_ref() { self.busy.set(true); f(args); self.busy.set(false); } }
        self.inner.run(args, input)
    }
    fn kids(&self, pid: i64) -> Vec<i64> { self.inner.kids(pid) }
    fn on_terminal(&self, pid: i64) -> Option<bool> { self.inner.on_terminal(pid) }
    fn kill(&self, pid: i64) { self.inner.kill(pid) }
}
/// A review that waits for the test: the reviewer takes its brief and works; you chat with the agent then (`chat`),
/// and then the reviewer writes its verdict on the commit it was given.
fn review_with_a_chat(w: &World, id: &str, chat: impl Fn() + 'static) -> Rc<RefCell<String>> {
    let said: Rc<RefCell<String>> = Rc::default();
    let h = Rc::new(Hooked { inner: w.fake.clone(), on: RefCell::new(None), busy: Cell::new(false) });
    sw::herdr::cli::set_fake(Some(h.clone()));
    let (id, step, said2, fake) = (id.to_string(), Rc::new(Cell::new(0)), said.clone(), w.fake.clone());
    let brief_taken = move |t: &Obj| { let pane = t.at("link").at("reviewer").t("pane"); fake.st.borrow().arr("panes").iter().any(|p| p.s("pane_id") == pane && !p.at("agent").arr("prompts").is_empty()) };
    *h.on.borrow_mut() = Some(Box::new(move |_| {
        let t = task(&id);
        if t.s("stage") != "reviewing" { return; }
        if step.get() == 0 && brief_taken(&t) { step.set(1); sw::clock::advance(1_000); *said2.borrow_mut() = format!("{} | {}", sw::actions::talk::reply(&id, "Why v1?", "you").map(|s| s.msg).unwrap_or_else(|e| e.msg), task(&id).s("stage")); chat(); }
        else if step.get() == 1 {
            step.set(2);
            let head = t.at("chat").t("head");
            let v = json!({ "meets": true, "findings": [], "text": "Looks right.", "commit": head });
            let dir = sw::flow::task::review_dir(&t); std::fs::create_dir_all(&dir).unwrap(); std::fs::write(dir.join("verdict.json"), v.to_string()).unwrap();
        }
    }));
    said
}

#[test]
fn a_review_under_way_goes_on_through_a_chat_that_changes_no_code() {
    let w = world();
    hold_clock();
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }, { "note": "Because the task asks for v1." }], "reviewer": [{ "status": "working" }] }));
    let id = w.add("Make f");
    let said = review_with_a_chat(&w, &id, || {});
    delegate(&id);
    let t = settle(&id);
    sw::herdr::cli::set_fake(Some(w.fake.clone()));
    assert_eq!(said.borrow().as_str(), "Sent to the agent: the review goes on unless the code changes | reviewing");
    assert!(prompts(&w, t.at("link").s("pane")).iter().any(|p| p.contains("Why v1?")), "the words reached the agent");
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!((reviews_started(&w), t.i("reviewRound")), (1, 1), "the one review, finished");
    assert!(!t.at("review").b("stale") && !t.b("chat") && !t.b("requireApproval"), "{}", t.at("chat"));
    assert!(says(&id).ends_with("Chatting · no changes, review kept"), "{}", says(&id));
    sw::clock::advance(10 * 60_000); housekeeping(&w);
    assert_eq!((settle(&id).s("stage"), reviews_started(&w)), ("approval".into(), 1));
}

#[test]
fn a_chat_that_changes_the_code_under_review_is_reviewed_again_once_you_are_done() {
    let w = world();
    hold_clock();
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }, { "files": { "f.txt": "v2\n" }, "note": "Made it v2." }], "reviewer": [{ "status": "working" }] }));
    let id = w.add("Make f");
    review_with_a_chat(&w, &id, || {});
    delegate(&id);
    let t = settle(&id);
    sw::herdr::cli::set_fake(Some(w.fake.clone()));
    // the review of v1 ended, and what the agent changed meanwhile was not put back: it waits for you
    let tree = std::path::PathBuf::from(t.s("worktree"));
    assert_eq!(read(&tree.join("f.txt")), "v2\n");
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure")); assert!(t.at("chat").b("held") && t.at("review").b("stale"), "{}", t.at("chat"));
    assert_eq!(reviews_started(&w), 1);
    sw::clock::advance(61_000); housekeeping(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!((reviews_started(&w), t.i("reviewRound")), (2, 2));
    assert_eq!(t.s("reviewedHead"), git(&tree, &["rev-parse", "HEAD"])); assert_eq!(git(&tree, &["show", "HEAD:f.txt"]), "v2");
}

#[test]
fn an_approved_task_whose_code_changes_loses_its_approval_and_never_merges_unreviewed() {
    let w = world_with(Opts { repo: true, origin: true, ..Default::default() });
    let id = to_approval(&w, "Waits for CI");
    hold_clock();
    let head = task(&id).t("prHead");
    let running = json!({ "state": "OPEN", "headRefOid": head, "statusCheckRollup": [{ "__typename": "CheckRun", "name": "build", "status": "IN_PROGRESS", "conclusion": "" }], "comments": [], "reviews": [], "author": { "login": "me" } });
    w.gh_says(json!({ "view": running }));
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging"); assert!(t.b("mergeIntent"));
    // you ask for one more change while it waits for GitHub
    w.script(json!({ "worker": [{ "files": { "f.txt": "v2\n" }, "note": "v2 now." }] }));
    tell(&id, "Make it v2 after all");
    let t = settle(&id);
    assert!(!t.b("mergeIntent") && t.b("requireApproval"), "the approval is withdrawn: {}", t.at("mergeIntent"));
    // the change waits for you to be done, as any chat's does
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure")); assert!(t.at("chat").b("held"), "{}", t.at("chat"));
    assert_eq!((t.i("reviewRound"), reviews_started(&w)), (1, 1), "no review yet");
    assert_eq!(row(&id).s("msg"), "Changes made · review starts when you're done");
    sw::clock::advance(59_000); housekeeping(&w); settle(&id);
    assert_eq!((task(&id).s("stage"), reviews_started(&w)), ("working".into(), 1));
    sw::clock::advance(2_000); housekeeping(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(t.i("reviewRound") == 2 && reviews_started(&w) == 2 && !t.at("review").b("stale"), "the change is reviewed, once");
    // GitHub's checks pass now: still nothing merges without your new yes
    let new_head = task(&id).t("prHead");
    w.gh_says(json!({ "view": { "state": "OPEN", "headRefOid": new_head, "statusCheckRollup": [{ "__typename": "CheckRun", "name": "build", "status": "COMPLETED", "conclusion": "SUCCESS" }], "comments": [], "reviews": [], "author": { "login": "me" } } }));
    sw::housekeeping::poll_prs(); housekeeping(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert!(!gh_log(&w).lines().any(|l| l.starts_with("pr merge") && !l.contains("--disable-auto")), "{}", gh_log(&w));
}
