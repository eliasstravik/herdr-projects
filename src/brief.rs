//! Getting a thread's brief line to its agent, once, and only when it is
//! really ready. Every sender (the ticker's passes, `thread brief`, adoption)
//! goes through `deliver`.
//!
//! - Ready means idle or done, the input box readable and empty, and (for the
//!   ticker) the state and screen unchanged for `SETTLE_SECS`: a start-up
//!   screen that reads idle for a moment does not get the text.
//! - Sent means herdr saw the agent start working (`--wait --until working`),
//!   not that `agent prompt` exited 0.
//! - After any try that may have typed, the screen decides: the line in the
//!   conversation is delivered, the line left in the box gets Enter, an empty
//!   box with no line is typed again. Anything else waits. Text is never
//!   typed on top of a copy.
//! - `MAX_ATTEMPTS` tries, then one inbox item; `thread brief` still sends it.

use anyhow::Result;

use crate::herdr::{Agent, Herdr};
use crate::project::{self, Project};
use crate::prompt_box;
use crate::thread::{self, Thread};
use crate::inbox;

pub const SETTLE_SECS: i64 = 3;
pub const MAX_ATTEMPTS: u32 = 3;
/// A claim older than this belongs to a sender that died.
pub const CLAIM_SECS: i64 = 60;

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// herdr saw the agent start on it, or the screen shows it was taken.
    Delivered,
    /// Not yet; the reason, for `thread brief`.
    Waiting(String),
    /// Another sender is at it right now, or it was delivered already.
    Taken,
    /// The tries ran out: an inbox item was written.
    Stuck,
    /// The pane shows a trust screen (this line of it): nothing is typed, as
    /// its Enter would accept the screen.
    TrustScreen(&'static str),
}

/// Who is sending: the ticker waits for a settled screen and stops after
/// `MAX_ATTEMPTS`; a person (`thread brief`) or adoption does neither.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sender {
    Ticker,
    Manual,
}

/// The part of the launch line that is always on one screen line.
fn marker(slug: &str, id: &str) -> String {
    format!(".herdr-project/{slug}-{id}/brief.md")
}

fn squeeze(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Claims the brief under the project lock; `None` when it is not pending
/// any more or another sender's claim is fresh.
fn claim(project: &Project, id: &str, now: jiff::Timestamp) -> Result<Option<Thread>> {
    let mut claimed = None;
    thread::update(project, id, |t| {
        let fresh = !t.brief_claimed.is_empty() && thread::seconds_since(&t.brief_claimed, now) < CLAIM_SECS;
        if t.prompt_pending && !fresh {
            t.brief_claimed = project::now();
            claimed = Some(t.clone());
        }
    })?;
    Ok(claimed)
}

/// One delivery step for a thread whose brief is pending, `agent` being its
/// agent as just listed.
pub fn deliver(project: &Project, herdr: &Herdr, record: &Thread, agent: &Agent, sender: Sender) -> Result<Outcome> {
    let now = jiff::Timestamp::now();
    let Some(mut t) = claim(project, &record.id, now)? else {
        return Ok(Outcome::Taken);
    };
    let outcome = step(project, herdr, &mut t, agent, sender, now);
    let delivered = matches!(outcome, Ok(Outcome::Delivered));
    thread::update(project, &t.id, |r| {
        r.brief_claimed.clear();
        r.brief_attempts = t.brief_attempts;
        r.brief_seen = t.brief_seen.clone();
        r.brief_seen_at = t.brief_seen_at.clone();
        r.brief_stuck = t.brief_stuck;
        if delivered {
            r.prompt_pending = false;
            r.brief_stuck = false;
        }
    })?;
    outcome
}

fn step(project: &Project, herdr: &Herdr, t: &mut Thread, agent: &Agent, sender: Sender, now: jiff::Timestamp) -> Result<Outcome> {
    let state = agent.agent_status.as_str();
    if matches!(state, "blocked" | "unknown" | "") {
        t.brief_seen.clear();
        return Ok(Outcome::Waiting(format!("the agent is {}", if state.is_empty() { "not detected" } else { state })));
    }
    let ready = agent.ready();
    // Nothing typed yet and not ready: nothing to look at.
    if t.brief_attempts == 0 && !ready {
        t.brief_seen.clear();
        return Ok(Outcome::Waiting(format!("the agent is {state}")));
    }
    let screen = match herdr.agent_screen(&t.pane_id) {
        Ok(screen) => screen,
        Err(error) => return Ok(Outcome::Waiting(format!("its screen could not be read: {error}"))),
    };
    let kind = if agent.agent.is_empty() { t.agent.as_str() } else { agent.agent.as_str() };
    if let Some(phrase) = crate::trust_screen::detect(kind, &screen) {
        t.brief_seen.clear();
        return Ok(Outcome::TrustScreen(phrase));
    }
    let known = prompt_box::knows(kind);
    let in_box = prompt_box::box_text(kind, &screen);
    let line = squeeze(&marker(&project.slug, &t.id));
    let shown = squeeze(&prompt_box::plain(&screen)).contains(&line);

    if t.brief_attempts > 0 {
        let in_box_has_line = in_box.as_deref().is_some_and(|text| squeeze(text).contains(&line));
        // On screen and not in the box: it was taken. An agent whose box
        // cannot be read counts only once it works.
        let taken = shown && !in_box_has_line && (in_box.as_deref() == Some("") || (!known && !ready));
        match in_box.as_deref() {
            _ if taken => return Ok(Outcome::Delivered),
            // Left in the box: submit it, never type it again.
            Some(_) if in_box_has_line && ready => {
                if let Some(stuck) = out_of_tries(project, t, sender)? {
                    return Ok(stuck);
                }
                bump(project, t)?;
                return Ok(match herdr.agent_send_keys(&t.pane_id, &["enter".to_string()]) {
                    Ok(()) => Outcome::Waiting("pressed Enter on the brief left in its input box".into()),
                    Err(error) => Outcome::Waiting(format!("could not press Enter: {error}")),
                });
            }
            // An empty box and no line anywhere: it was dropped.
            Some("") if ready => {}
            // A box this binary cannot read: only a person decides.
            None if !known => {
                if sender == Sender::Manual {
                    if shown {
                        // They looked; the line is on screen.
                        return Ok(Outcome::Delivered);
                    }
                } else {
                    return Ok(give_up(project, t, "its input box cannot be read to check an earlier try")?);
                }
            }
            _ => return Ok(Outcome::Waiting(format!("the agent is {state} and an earlier try may still be in its input box"))),
        }
    } else if known && in_box.as_deref() != Some("") {
        t.brief_seen.clear();
        let why = if in_box.is_none() { "its input box is not on screen (a menu or start-up screen?)" } else { "its input box holds text" };
        return Ok(Outcome::Waiting(why.into()));
    }

    if let Some(stuck) = out_of_tries(project, t, sender)? {
        return Ok(stuck);
    }
    if sender == Sender::Ticker {
        let key = format!("{}:{:x}", agent.state_change_seq, fnv(&prompt_box::plain(&screen)));
        if t.brief_seen != key {
            t.brief_seen = key;
            t.brief_seen_at = project::now();
            return Ok(Outcome::Waiting("settling".into()));
        }
        if thread::seconds_since(&t.brief_seen_at, now) < SETTLE_SECS {
            return Ok(Outcome::Waiting("settling".into()));
        }
    }
    bump(project, t)?;
    match herdr.agent_prompt_confirmed(&t.pane_id, &thread::launch_prompt(&project.slug, &t.id)) {
        Ok(()) => Ok(Outcome::Delivered),
        Err(error) if crate::herdr::refused_before_typing(&error) => {
            t.brief_attempts -= 1;
            Ok(Outcome::Waiting(format!("{error}")))
        }
        Err(error) => Ok(Outcome::Waiting(format!("not confirmed ({error}); the screen is checked before any retry"))),
    }
}

/// Counts a try before it is made, so a sender that dies mid-way still
/// leaves the record saying text may be on screen.
/// A retry waits for a fresh settle.
fn bump(project: &Project, t: &mut Thread) -> Result<()> {
    t.brief_attempts += 1;
    t.brief_seen.clear();
    let attempts = t.brief_attempts;
    thread::update(project, &t.id, |r| r.brief_attempts = attempts)?;
    Ok(())
}

fn out_of_tries(project: &Project, t: &mut Thread, sender: Sender) -> Result<Option<Outcome>> {
    if sender == Sender::Ticker && t.brief_attempts >= MAX_ATTEMPTS {
        return give_up(project, t, &format!("{MAX_ATTEMPTS} tries were not confirmed")).map(Some);
    }
    Ok(None)
}

fn give_up(project: &Project, t: &mut Thread, why: &str) -> Result<Outcome> {
    if !t.brief_stuck {
        t.brief_stuck = true;
        let summary = format!(
            "{}: its brief did not get through ({why}). `thread read {} {}` shows the pane; `thread brief` sends it once the input box is empty",
            t.id, project.slug, t.id
        );
        inbox::write(project, "thread-state", &t.id, "brief not delivered", &summary, "")?;
    }
    Ok(Outcome::Stuck)
}

/// A small stable hash of the screen text (FNV-1a).
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf29ce484222325, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::runner::fake::{fail, ok};
    use crate::scenarios::{World, agent_json, claude_screen, pane_json, prompt_text, settled};
    use crate::thread::Group;
    use crate::ticker::brief_pass_for_test as pass;

    const STALLED: &str = r#"{"error":{"code":"agent_prompt_stalled","message":"agent did not start working"}}"#;

    /// A started thread waiting for its brief in pane `w2:p1`, its agent
    /// `state`; `agent prompt` answers with whatever `reply` holds.
    fn world(state: &str) -> (World, Project, Rc<RefCell<std::result::Result<String, String>>>) {
        let world = World::new();
        let project = world.project("demo", "a.sock");
        let cwd = world.home.path().to_string_lossy().into_owned();
        world.thread(&project, world.home.path(), |t| {
            t.prompt_pending = true;
            t.launch_attempts = 1;
            t.launched_at = project::now();
        });
        *world.panes.borrow_mut() = format!("[{},{}]", world.coordinator_pane(&project), pane_json("w2", "w2:t1", "w2:p1", &cwd));
        set_state(&world, state);
        let reply: Rc<RefCell<std::result::Result<String, String>>> = Rc::new(RefCell::new(Ok(r#"{"result":{"agent":{"agent_status":"working"}}}"#.to_string())));
        let answer = reply.clone();
        world.runner.on_fn(
            |cmd| cmd.display().contains("agent prompt"),
            move |_| Ok(match &*answer.borrow() {
                Ok(text) => ok(text),
                Err(text) => fail(1, text),
            }),
        );
        world.runner.on("agent send-keys", ok(""));
        (world, project, reply)
    }

    fn set_state(world: &World, state: &str) {
        let cwd = world.home.path().to_string_lossy().into_owned();
        *world.agents.borrow_mut() = format!("[{}]", agent_json("w2", "w2:t1", "w2:p1", &cwd, "hp-demo-t-0001", state));
    }

    fn line() -> String {
        thread::launch_prompt("demo", "t-0001")
    }

    /// The launch line in the conversation above an empty box.
    fn taken_screen() -> String {
        format!("❯ {}\n\n● Reading the brief\n{}", line(), claude_screen(None))
    }

    fn record(project: &Project) -> Thread {
        thread::load(project, "t-0001").unwrap()
    }

    fn prompts(world: &World) -> usize {
        world.runner.count("agent prompt")
    }

    #[test]
    fn a_start_up_screen_that_reads_idle_gets_nothing_until_the_box_shows_and_settles() {
        let (world, project, _) = world("idle");
        let ctx = world.ctx();
        // A start-up screen Herdr reads as idle: no input box yet, so nothing
        // is typed (a trust screen is held the same way, by `trust_screen`).
        *world.screen.borrow_mut() = "Starting up\n  model: loading\n".into();
        for _ in 0..3 {
            assert!(pass(&ctx));
            settled(&project);
        }
        assert_eq!(prompts(&world), 0);
        // The box shows: seen once, then sent only after it stayed the same.
        *world.screen.borrow_mut() = claude_screen(None);
        assert!(pass(&ctx));
        assert_eq!(prompts(&world), 0);
        assert!(pass(&ctx), "not settled yet");
        assert_eq!(prompts(&world), 0);
        // The screen changed in between (a redraw): the wait starts again.
        *world.screen.borrow_mut() = format!("tips\n{}", claude_screen(None));
        settled(&project);
        assert!(pass(&ctx));
        assert_eq!(prompts(&world), 0);
        settled(&project);
        assert!(!pass(&ctx));
        assert_eq!(prompts(&world), 1);
        let calls = world.runner.calls.borrow();
        let sent = calls.iter().find(|c| c.display().contains("agent prompt")).unwrap();
        assert_eq!(prompt_text(sent), line());
        assert!(sent.display().ends_with("--wait --until working --until blocked --timeout 8000"), "{}", sent.display());
        drop(calls);
        let t = record(&project);
        assert!(!t.prompt_pending && t.brief_claimed.is_empty());
    }

    #[test]
    fn a_stalled_brief_left_in_the_box_gets_enter_never_a_second_copy() {
        let (world, project, reply) = world("idle");
        let ctx = world.ctx();
        *reply.borrow_mut() = Err(STALLED.into());
        pass(&ctx);
        settled(&project);
        assert!(pass(&ctx));
        assert_eq!(prompts(&world), 1);
        let t = record(&project);
        assert!(t.prompt_pending);
        assert_eq!(t.brief_attempts, 1);
        // The line sits in the box: Enter, not the text again.
        *world.screen.borrow_mut() = claude_screen(Some(&line()));
        assert!(pass(&ctx));
        assert_eq!((prompts(&world), world.runner.count("agent send-keys w2:p1 enter")), (1, 1));
        // Now it is in the conversation and the agent works: delivered.
        set_state(&world, "working");
        *world.screen.borrow_mut() = taken_screen();
        assert!(!pass(&ctx));
        let t = record(&project);
        assert!(!t.prompt_pending);
        assert_eq!((prompts(&world), world.runner.count("send-keys")), (1, 1));
        // The next tick sends nothing either.
        crate::ticker::tick_project(&ctx, &project).unwrap();
        assert_eq!(prompts(&world), 1);
    }

    #[test]
    fn a_dropped_brief_is_typed_again_up_to_the_cap_then_one_item_and_waiting_on_you() {
        let (world, project, reply) = world("idle");
        let ctx = world.ctx();
        *reply.borrow_mut() = Err(STALLED.into());
        // Each try: seen, settled, typed; the box stays empty and the line
        // never shows, so the text was dropped and may be typed again.
        for n in 1..=MAX_ATTEMPTS as usize {
            pass(&ctx);
            settled(&project);
            pass(&ctx);
            assert_eq!(prompts(&world), n);
        }
        settled(&project);
        assert!(!pass(&ctx), "stuck: no more looks");
        assert_eq!(prompts(&world), MAX_ATTEMPTS as usize);
        let t = record(&project);
        assert!(t.prompt_pending && t.brief_stuck);
        let items: Vec<_> = inbox::unhandled(&project).into_iter().filter(|i| i.event == "brief not delivered").collect();
        assert_eq!(items.len(), 1);
        assert!(items[0].summary.contains("`thread brief`"), "{}", items[0].summary);
        // The ticker leaves it alone and shows it as waiting on you.
        crate::ticker::tick_project(&ctx, &project).unwrap();
        assert_eq!(prompts(&world), MAX_ATTEMPTS as usize);
        assert_eq!(record(&project).last_group, Group::WaitingOnYou.token());
        // `thread brief` still sends it, at once.
        *reply.borrow_mut() = Ok(r#"{"result":{}}"#.into());
        crate::threads::brief(&ctx, "demo", "t-0001").unwrap();
        let t = record(&project);
        assert!(!t.prompt_pending && !t.brief_stuck);
        assert_eq!(prompts(&world), MAX_ATTEMPTS as usize + 1);
    }

    #[test]
    fn a_draft_in_the_box_or_an_unreadable_screen_after_a_try_waits() {
        let (world, project, reply) = world("idle");
        let ctx = world.ctx();
        *reply.borrow_mut() = Err(r#"{"error":{"code":"timeout","message":"slow"}}"#.into());
        pass(&ctx);
        settled(&project);
        pass(&ctx);
        assert_eq!(prompts(&world), 1);
        // Someone else's text: neither typed over nor submitted.
        *world.screen.borrow_mut() = claude_screen(Some("my own note"));
        for _ in 0..3 {
            settled(&project);
            assert!(pass(&ctx));
        }
        // A menu: nothing.
        *world.screen.borrow_mut() = "Pick one\n❯ 1. Yes\n".into();
        settled(&project);
        assert!(pass(&ctx));
        assert_eq!((prompts(&world), world.runner.count("send-keys")), (1, 0));
        assert!(record(&project).prompt_pending);
    }

    #[test]
    fn a_blocked_answer_types_nothing_and_does_not_count() {
        let (world, project, reply) = world("idle");
        let ctx = world.ctx();
        *reply.borrow_mut() = Err(r#"{"error":{"code":"agent_blocked","message":"blocked"}}"#.into());
        pass(&ctx);
        settled(&project);
        pass(&ctx);
        assert_eq!(record(&project).brief_attempts, 0);
    }

    #[test]
    fn a_fresh_claim_is_left_alone_and_a_dead_one_is_checked_on_screen() {
        let (world, project, _) = world("idle");
        let ctx = world.ctx();
        thread::update(&project, "t-0001", |t| t.brief_claimed = project::now()).unwrap();
        for _ in 0..2 {
            settled(&project);
            pass(&ctx);
        }
        assert_eq!(prompts(&world), 0);
        // A sender died after typing (its try counted): the line is in the
        // conversation, so it counts as delivered without typing.
        thread::update(&project, "t-0001", |t| {
            t.brief_claimed = "2026-01-01T00:00:00Z".into();
            t.brief_attempts = 1;
        })
        .unwrap();
        set_state(&world, "working");
        *world.screen.borrow_mut() = taken_screen();
        assert!(!pass(&ctx));
        assert!(!record(&project).prompt_pending);
        assert_eq!(prompts(&world), 0);
    }

    #[test]
    fn a_kind_whose_box_cannot_be_read_gets_one_try_then_goes_to_a_person() {
        let (world, project, reply) = world("idle");
        let ctx = world.ctx();
        thread::update(&project, "t-0001", |t| t.agent = "copilot".into()).unwrap();
        let cwd = world.home.path().to_string_lossy().into_owned();
        *world.agents.borrow_mut() = format!("[{}]", agent_json("w2", "w2:t1", "w2:p1", &cwd, "hp-demo-t-0001", "idle").replace("\"claude\"", "\"copilot\""));
        *world.screen.borrow_mut() = "copilot> \n".into();
        *reply.borrow_mut() = Err(STALLED.into());
        pass(&ctx);
        settled(&project);
        pass(&ctx);
        assert_eq!(prompts(&world), 1);
        settled(&project);
        assert!(!pass(&ctx));
        assert_eq!(prompts(&world), 1);
        assert!(record(&project).brief_stuck);
    }

    #[test]
    fn a_remote_brief_goes_out_between_machine_polls() {
        let (world, project, _) = world("idle");
        let ctx = world.ctx();
        thread::update(&project, "t-0001", |t| t.machine = "box".into()).unwrap();
        pass(&ctx);
        settled(&project);
        assert!(!pass(&ctx));
        let calls = world.runner.calls.borrow();
        let sent = calls.iter().find(|c| c.display().contains("agent prompt")).unwrap();
        assert_eq!(sent.args[..2], ["--machine".to_string(), "box".to_string()]);
        drop(calls);
        assert!(!record(&project).prompt_pending);
        // Long after its start, a remote brief waits for the machine's own poll.
        thread::update(&project, "t-0001", |t| {
            t.prompt_pending = true;
            t.launched_at = "2026-01-01T00:00:00Z".into();
        })
        .unwrap();
        let before = world.runner.calls.borrow().len();
        assert!(!pass(&ctx));
        assert_eq!(world.runner.calls.borrow().len(), before);
    }

    #[test]
    fn an_unconfirmed_follow_up_is_reported_and_not_sent_again() {
        let (world, project, reply) = world("working");
        let ctx = world.ctx();
        thread::update(&project, "t-0001", |t| t.prompt_pending = false).unwrap();
        *reply.borrow_mut() = Err(STALLED.into());
        let error = crate::threads::prompt(&ctx, "demo", "t-0001", "Also the docs.").unwrap_err().to_string();
        assert!(error.contains("prompt_unconfirmed") && error.contains("Do not send it again"), "{error}");
        assert_eq!(prompts(&world), 1);
        // It was typed, so the task file records it.
        assert!(std::fs::read_to_string(thread::task_path(&project, "t-0001")).unwrap().contains("Also the docs."));
        // A refusal before typing records nothing.
        *reply.borrow_mut() = Err(r#"{"error":{"code":"agent_blocked","message":"blocked"}}"#.into());
        assert!(crate::threads::prompt(&ctx, "demo", "t-0001", "Second.").is_err());
        assert!(!std::fs::read_to_string(thread::task_path(&project, "t-0001")).unwrap().contains("Second."));
    }
}
