//! What the board asks to be done, done away from the keyboard: each job runs on a thread of its own and answers with
//! what to say and what follows (`Done`). A job carries the words the board says when it worked, worked out where the
//! key was pressed (the task's name as its row has it), so this file only does and reports.
use crate::actions::{archive, filing, finish_line, limits, start, talk};
use crate::json::*;
use crate::shown::ink::Ink;

#[derive(Clone, Debug, PartialEq)]
pub enum Job {
    /// A message to a task's agent (or one that starts it). `fresh`: the task had no agent yet.
    Tell { id: String, text: String, fresh: bool },
    /// `now`: a task that waits for others (after.rs) starts without waiting.
    Delegate { id: String, name: String, now: bool },
    Approve { id: String, seen: Option<(Value, Value)>, anyway: bool },
    FinishAnyway { id: String }, Retry { id: String }, CarryOn { id: String },
    SendFailure { id: String }, SendError { id: String }, SendReview { id: String },
    Stop { id: String, said: String },
    Pin { id: String, name: String, key: String },
    Archive { id: String, said: String }, Restore { id: String, said: String },
    /// This machine's project `repo` with its tasks, to the archive and back; `said` follows what it says it did.
    ArchiveProject { repo: String, said: String }, RestoreProject { repo: String, said: String },
    Clear { id: String, name: String },
    Pick { id: String, letter: String },
    /// What you typed, for this machine's Coordinator; with no Coordinator, a task (or words added to one's description).
    CoordSend { cwd: String, text: String, delegate: bool, about: String, to: String },
    Add { cwd: String, text: String, delegate: bool },
    Describe { id: String, text: String },
    /// Words for another machine's Coordinator: a row of their own until that machine has them.
    Dispatch { machine: crate::remote::Machine, text: String, delegate: bool, about: String, to: String },
    StopDispatch { id: String },
    /// One of the exact actions, on a task or an automation of another machine.
    Far { machine: crate::remote::Machine, host: String, action: String, req: Value, quiet: bool },
    AutoTurn { id: String, on: bool }, AutoFire { id: String }, AutoRemove { id: String, name: String },
    /// A coordinator of this machine (`to`, as the coordinator code names its level) on autopilot, or off (autopilot/).
    Pilot { to: String, on: bool },
    /// A coordinator of this machine started (in its own conversation) or stopped, as an agent on a task is.
    CoordRun { to: String, name: String, on: bool },
    /// Going into a coordinator of this machine (enter on its row): started first when it is not running.
    CoordOpen { to: String, name: String, popup: bool },
    /// A setting of a task, an automation, yours or your entry for a repo (`whose`): a value, or none to put it back.
    Set { id: String, name: String, value: Option<String>, whose: super::state::Whose },
    CoordSeen { unseen: String, to: String },
    /// You left the chat with a task whose chat changed its code: its review need not wait the quiet time.
    ChatLeft { id: String },
    /// Into a task's agent: its real herdr pane (`attempt`: the selected attempt's, not the task's).
    OpenAgent { id: String, popup: bool },
    FreeSize { pids: Vec<i64> },
    /// The first run's answers: which blocks to add to herdr's config.
    Setup { key: Option<String>, sidebar: Option<bool>, file: String },
    /// The web board's "Open agent": the task's space reopened (on its machine: the machine, its label, the web UI's
    /// PC for it and the pane it last named), and the web UI's link to its pane.
    WebOpen { id: String, far: Option<(crate::remote::Machine, String, String, String)> },
    /// The herdr web UI's link through the board's front door, or left alone for good.
    WebUiLink, WebUiDecline,
    /// A voice call for whoever the chat talks to (calls/ring.rs), hanging it up, and who it talks to now.
    CallRing { owner: String, target: crate::voice::call::Target, session: String }, CallEnd, CallTarget { target: crate::voice::call::Target },
    Nothing,
}
impl Job {
    /// The row this job acts on: a task's id (another machine's as `host:id`), an automation's row; "" for the rest.
    pub fn about(&self) -> String {
        match self {
            Job::Tell { id, .. } | Job::Delegate { id, .. } | Job::Approve { id, .. } | Job::FinishAnyway { id } | Job::Retry { id } | Job::CarryOn { id }
            | Job::SendFailure { id } | Job::SendError { id } | Job::SendReview { id } | Job::Stop { id, .. } | Job::Pin { id, .. } | Job::Archive { id, .. }
            | Job::Restore { id, .. } | Job::Clear { id, .. } | Job::Pick { id, .. } | Job::Describe { id, .. } | Job::OpenAgent { id, .. } => id.clone(),
            Job::AutoTurn { id, .. } | Job::AutoFire { id } | Job::AutoRemove { id, .. } => format!("auto:{id}"),
            Job::Far { host, action, req, .. } if action.starts_with("auto-") => format!("{host}:auto:{}", req.s("id")),
            // A coordinator's row, by its coordinator (coords.rs: Target::key): started, stopped, autopilot, gone into.
            Job::CoordRun { to, .. } | Job::Pilot { to, .. } | Job::CoordOpen { to, .. } => format!("co:\t{to}"),
            // A project archived or restored: its row, the project coordinator's.
            Job::ArchiveProject { repo, .. } | Job::RestoreProject { repo, .. } => format!("co:\tproject:{repo}"),
            Job::Far { host, action, req, .. } if action.starts_with("coordinator-") || action.starts_with("autopilot-") => format!("co:{host}\t{}", req.s("to")),
            Job::Far { host, req, .. } => format!("{host}:{}", req.s("id")),
            _ => String::new(),
        }
    }
    /// Which job this is, to tell two jobs on one row apart: its variant ("Stop"), or another machine's action ("far:stop").
    pub fn kind(&self) -> String {
        match self { Job::Far { action, .. } => format!("far:{action}"), j => format!("{j:?}").split([' ', '{', '(']).next().unwrap_or("").to_string() }
    }
}
/// What follows a job, for the board to take in.
#[derive(Clone, Debug, PartialEq)]
pub enum Then {
    /// Select this task (one that was just added).
    Sel(String),
    /// A task was archived: the undo key brings it back.
    Archived(String),
    /// A task left the list (archived, or restored out of the archive): the message says where the selection went.
    Left(String),
    /// Nothing was sent: the words go back into the box, or are put away when the box has others by now.
    Unsent { text: String, to: String },
    /// The board is in herdr's popup and an agent was opened: it closes.
    Leave,
    /// This terminal becomes the agent's until you detach (outside a herdr client, or another machine's agent on an
    /// older plugin).
    Attach { command: String, args: Vec<String>, id: String },
    /// An agent of this machine was opened in herdr, and the board stays where it is.
    Opened(String),
    /// Another machine's herdr put this task's agent (`agent`; else its bare terminal) in front: the board stays and
    /// says to pick that machine in herdr's sidebar.
    OpenedThere { id: String, host: String, agent: bool },
    /// A paste-it-yourself text the first run could not write.
    Paste(String),
    /// The first run: which of its two choices are still to be asked (a refused one); both settled: the screen goes.
    SetupLeft { key: bool, sidebar: bool },
    /// The web page goes to the web UI's view of this pane.
    Go(Value),
    /// The open automation was removed: nothing of it stays open.
    AutoGone,
    /// A setting typed on the settings page was taken: the box it was typed in is emptied. A refused one stays there
    /// to be put right.
    Saved(String),
    /// A call rang: its id, for the chat to say when no helper took it, or why it ended before it began.
    CallRang { id: String },
}
/// `about`: the row the job acted on (Job::about), `kind` which job it was (Job::kind); `seen`: how many reads of the tasks had started when it ended, so the
/// board knows which read shows what it did (update/pending.rs).
#[derive(Clone, Debug, PartialEq)]
pub struct Done { pub flash: Option<(String, Ink)>, pub then: Vec<Then>, pub ok: bool, pub about: String, pub kind: String, pub seen: u64 }
/// Done, and nothing to say: it worked.
impl Default for Done { fn default() -> Self { Done { flash: None, then: vec![], ok: true, about: String::new(), kind: String::new(), seen: 0 } } }
impl Done {
    pub fn say(text: impl Into<String>, ink: Ink) -> Done { Done { flash: Some((text.into(), ink)), ..Default::default() } }
    pub fn fail(text: impl Into<String>) -> Done { Done { flash: Some((text.into(), Ink::Red)), ok: false, ..Default::default() } }
    pub fn and(mut self, t: Then) -> Done { self.then.push(t); self }
}
/// Run one job to its end. Errors are said in red, as the board says every error.
/// Words that did not go out are never lost: the board puts them back.
pub fn run(job: Job) -> Done {
    let unsent = match &job {
        Job::Tell { id, text, .. } => Some((text.clone(), id.clone())),
        Job::CoordSend { text, about, .. } => Some((text.clone(), about.clone())),
        Job::Add { text, .. } => Some((text.clone(), String::new())),
        Job::Describe { id, text } => Some((text.clone(), id.clone())),
        Job::Far { host, action, req, .. } if action == "tell" => Some((req.t("text"), format!("{host}:{}", req.s("id")))),
        _ => None,
    };
    let (about, kind) = (job.about(), job.kind());
    let mut d = run_one(job);
    d.about = about; d.kind = kind; d.seen = super::data::reads_started();
    if !d.ok { if let Some((text, to)) = unsent { d.then.push(Then::Unsent { text, to }); } }
    d
}
fn run_one(job: Job) -> Done {
    let green = |m: String| Done::say(m, Ink::Green);
    let out = (|| -> crate::err::Res<Done> {
        Ok(match job {
            Job::Tell { id, text, fresh } => {
                let out = talk::reply(&id, &text, crate::status_words::who::YOU)?;
                let ink = if out.msg.starts_with("queued") { Ink::Yellow } else { Ink::Green };
                let mut d = Done::say(out.msg.clone(), ink);
                if !fresh && out.msg == "Sent to the agent" { d.flash = Some((out.msg, Ink::Plain)); } // plainly sent: the box shows your words (said only with the chat hidden)
                d
            }
            Job::Delegate { id, name, now } => {
                let t = start::delegate_with(&id, vec![], crate::after::Wait { now, ..Default::default() })?;
                green(if let Some(w) = crate::after::waiting_words(&t) { format!("{name} {w}.") } else if t.s("stage") == "attempts" { format!("{} attempts are on {name}.", t.arr("attempts").len()) } else if t.s("stage") == "queued" { format!("{name} is {}.", crate::shown::queue::words(&crate::flow::live::saved(&t))) } else { format!("An agent is on {name}.") })
            }
            Job::Approve { id, seen, anyway } => green(finish_line::approve(&id, seen, anyway)?.msg),
            Job::FinishAnyway { id } => green(finish_line::finish_anyway(&id)?.msg),
            Job::Retry { id } => green(finish_line::retry(&id)?.msg),
            Job::CarryOn { id } => green(limits::carry_on(&id, false)?.msg),
            Job::SendFailure { id } => green(finish_line::send_failure(&id)?.msg),
            Job::SendError { id } => green(finish_line::send_error(&id)?.msg),
            Job::SendReview { id } => green(finish_line::send_review(&id)?.msg),
            Job::Stop { id, said } => { talk::stop(&id)?; Done::say(said, Ink::Yellow) }
            Job::Pin { id, name, key } => { let t = filing::pin(&id)?; Done::say(if t.b("pinned") { format!("Pinned {name} to the top. {key} unpins it.") } else { format!("Unpinned {name}.") }, Ink::Muted) }
            Job::Archive { id, said } => { archive::archive(&id)?; Done::say(said, Ink::Green).and(Then::Archived(id)) }
            Job::ArchiveProject { repo, said } => { let (r, _) = crate::actions::archive_project::archive_project(&repo)?; Done::say(format!("{r}{said}"), if r.contains("Still on the board") { Ink::Yellow } else { Ink::Green }) }
            Job::RestoreProject { repo, said } => { let (r, _) = crate::actions::archive_project::restore_project(&repo)?; Done::say(format!("{r}{said}"), Ink::Green) }
            Job::Restore { id, said } => { archive::restore(&id)?; Done::say(said, Ink::Green).and(Then::Left(id)) }
            Job::Clear { id, name } => {
                let (_, kept) = archive::clear(&id)?;
                if kept.is_empty() { Done::say(format!("Removed {name}."), Ink::Muted) } else { Done::say(format!("Removed {name}. Kept, so no work is lost: {}", kept.join(", ")), Ink::Yellow) }
            }
            Job::Pick { id, letter } => green(crate::attempts::settle::pick(&id, &letter)?.msg),
            Job::CoordSend { cwd, text, delegate, about, to } => { crate::coordinator::requests::send(std::path::Path::new(&cwd), &text, delegate, &about, crate::coordinator::co::Co::of(&crate::herdr::cli::current_socket(), &to))?; Done::default() }
            Job::Add { cwd, text, delegate } => {
                let (title, rest) = text.split_once('\n').unwrap_or((&text, ""));
                let t = filing::add(std::path::Path::new(&cwd), title, filing::AddOpts { description: rest.into(), ..Default::default() })?;
                if delegate { start::delegate(t.s("id"), vec![])?; }
                green(format!("{}: added{}", t.s("id"), if delegate { ", an agent is on it" } else { "" })).and(Then::Sel(t.t("id")))
            }
            Job::Describe { id, text } => {
                let t = crate::store::tasks::find(&id)?;
                let desc = if t.b("description") { format!("{}\n\n{text}", t.s("description")) } else { text };
                filing::describe(&id, &desc)?;
                green(format!("{id}: updated (description)"))
            }
            Job::Dispatch { machine, text, delegate, about, to } => super::job_far::dispatch(machine, text, delegate, about, to),
            Job::StopDispatch { id } => { crate::watcher::stop_dispatch(&id); Done::say("Stopped.", Ink::Yellow) }
            Job::Far { machine, host, action, req, quiet } => super::job_far::far(&machine, &host, &action, req, quiet),
            Job::AutoTurn { id, on } => { let (w, ink) = crate::automations::show::turned_words(&crate::automations::list::turn(&id, on)?); Done::say(w, ink) }
            Job::AutoFire { id } => { let a = crate::automations::list::find(&id)?; let f = crate::automations::fire::fire(&id, crate::automations::told::Cause::Run, crate::clock::now())?; let (w, ink) = crate::automations::show::fired_words(&a, &f); Done::say(w, ink) }
            Job::Pilot { to, on } => {
                let co = crate::coordinator::co::Co::of(&crate::herdr::cli::current_socket(), &to);
                // On says nothing: its heading shows it (view/pilot.rs).
                if on { crate::autopilot::switch::on(&co, None, &[])?; Done::default() } else { Done::say(crate::autopilot::switch::off(&co)?, Ink::Yellow) }
            }
            Job::CoordRun { to, name, on } => {
                let co = crate::coordinator::co::Co::of(&crate::herdr::cli::current_socket(), &to);
                if on { crate::coordinator::requests::restart(&co)?; Done::say(format!("Starting {name}, in its own conversation."), Ink::Muted) } else { crate::coordinator::requests::stop(&co)?; Done::say(format!("Stopped {name}. Your next message starts it again."), Ink::Muted) }
            }
            Job::CoordOpen { to, name, popup } => super::job_far::coord_open(&to, &name, popup),
            Job::AutoRemove { id, name } => { crate::automations::list::remove(&id)?; Done::say(format!("Removed the automation \"{name}\"."), Ink::Muted).and(Then::AutoGone) }
            Job::Set { id, name, value, whose } => super::job_far::set(&id, &name, value, &whose)?,
            Job::CoordSeen { unseen, to } => { crate::coordinator::requests::seen(&unseen, crate::coordinator::co::Co::of(&crate::herdr::cli::current_socket(), &to)); Done::default() }
            Job::ChatLeft { id } => { crate::finish::chat::left(&id); Done::default() }
            Job::OpenAgent { id, popup } => super::job_far::open_agent(&id, popup),
            Job::FreeSize { pids } => {
                let ended = pids.iter().filter(|p| unsafe { libc::kill(**p as i32, libc::SIGTERM) } == 0).count();
                if ended > 0 { Done::say(format!("Ended {}. The board has its own size again in a moment.", if ended == 1 { "the remote view of this pane".to_string() } else { format!("{ended} remote views of this pane") }), Ink::Green) }
                else { Done::say("The remote view could not be ended from here: close it where it runs.", Ink::Yellow) }
            }
            Job::Setup { key, sidebar, file } => super::job_far::setup(key, sidebar, &file),
            Job::WebOpen { id, far } => super::job_far::web_open(&id, far),
            Job::WebUiLink => { let r = crate::webui::command::link_now(); Done::say(r.t("msg"), if r.b("ok") { Ink::Green } else { Ink::Red }) }
            Job::WebUiDecline => { crate::webui::command::decline(); Done::say("Left as it is. `herdr-projects web-ui link` does it later.", Ink::Muted) }
            Job::CallRing { owner, target, session } => match crate::calls::ring::ring(&owner, &target, &session) {
                Ok(id) => Done::say(format!("Calling with {}…", target.name), Ink::Muted).and(Then::CallRang { id }),
                Err(e) => Done::fail(e),
            },
            Job::CallEnd => { crate::voice::call::end("hung up on the terminal board"); Done::say("Hung up.", Ink::Muted) }
            Job::CallTarget { target } => { crate::voice::call::set_target(&target); Done::default() }
            Job::Nothing => Done::default(),
        })
    })();
    out.unwrap_or_else(|e| Done::fail(e.msg))
}
