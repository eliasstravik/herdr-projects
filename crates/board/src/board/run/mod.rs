//! The board running in a terminal: the threads that read keys and the mouse, tick once a second, read the tasks and
//! run jobs; the loop that feeds each event to `update`, does what it answers and draws. Nothing slow runs on the loop:
//! a key is answered from what was last read, and what has to wait (herdr, git, another machine) is a thread's.
pub mod draw;
pub mod held;
pub mod input;
pub mod mouse;
pub mod place;
pub mod prefs;
pub mod say;
pub mod start;
pub mod term;

use crate::board::data::Wants;
use crate::board::update::{update, Effect, Event};
use crate::board::view::screen;
use crossterm::event as ce;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

pub enum Msg { Ev(Event), Mouse(ce::MouseEvent), Resize, Close, Load }
static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn on_signal(_: i32) { STOP.store(true, Ordering::SeqCst); }

/// What the loop keeps beside the board's state: the terminal, the theme, the threads' handles.
pub struct Run { pub tx: Sender<Msg>, pub wants: Arc<Mutex<Wants>>, pub load: Arc<(Mutex<bool>, std::sync::Condvar)>, pub machines_at: i64 }
impl Run {
    fn kick(&self) { let (m, c) = &*self.load; *m.lock().unwrap() = true; c.notify_one(); }
}
fn spawn_job(tx: &Sender<Msg>, j: crate::board::job::Job) { let tx = tx.clone(); std::thread::spawn(move || { let d = crate::board::job::run(j); let _ = tx.send(Msg::Ev(Event::Done(d))); }); }
fn poll(s: &mut crate::board::state::State, run: &mut Run, now_too: bool) {
    let now = crate::clock::now();
    if now - run.machines_at > 60000 { run.machines_at = now; let tx = run.tx.clone(); std::thread::spawn(move || { let _ = tx.send(Msg::Ev(Event::Machines(crate::remote::machines()))); }); }
    for (label, m) in s.remotes.iter_mut() {
        if m.busy || (!now_too && !m.ok && now - m.fail_at < 30000) { continue; }
        m.busy = true;
        let (tx, mc, label) = (run.tx.clone(), m.machine.clone(), label.clone());
        std::thread::spawn(move || { let a = crate::remote::ask_list(&mc, 20000); let _ = tx.send(Msg::Ev(Event::Remote { label, answer: a })); });
    }
}
pub fn open_board(folder: &std::path::Path) -> i32 {
    let (mut s, theme, named) = start::prepare(folder);
    let (tx, rx) = channel::<Msg>();
    let wants = Arc::new(Mutex::new(Wants { ids: vec![s.sel.clone()], cwd: s.ctx.cwd.clone(), own_pane: place::own_name(), web: false, coord: String::new(), mine: None, skills: String::new() }));
    let load = Arc::new((Mutex::new(true), std::sync::Condvar::new()));
    let mut run = Run { tx: tx.clone(), wants: wants.clone(), load: load.clone(), machines_at: 0 };
    start::threads(&tx, &wants, &load);
    unsafe { for sig in [libc::SIGTERM, libc::SIGHUP] { libc::signal(sig, on_signal as *const () as usize); } }
    term::guard_panics();
    if term::enter().is_err() { return 1; }
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let Ok(mut terminal) = ratatui::Terminal::new(backend) else { term::leave(false); return 1 };
    place::arrive();
    let mut sel = mouse::Selecting::default();
    let mut close = false;
    let mut reload = false;
    let (asking, mut woke_for) = (Arc::new(AtomicBool::new(false)), 0i64);
    if s.all() { poll(&mut s, &mut run, true); }
    loop {
        let size = terminal.size().unwrap_or_default();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::BeginSynchronizedUpdate);
        let (mut links, mut at) = (vec![], None);
        let _ = terminal.draw(|f| {
            let area = f.area();
            let drawn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| screen(&mut s, &theme, area.width as usize, area.height as usize)));
            let lines = match drawn { Ok(sc) => { at = sc.cursor; sc.lines } Err(_) => vec![crate::shown::ink::Inked::plain(" herdr-projects view error"), crate::shown::ink::Inked::plain(format!(" press {} to close", crate::board::keys::key_name("back")))] };
            links = draw::draw(&theme, &lines, f.buffer_mut());
            sel.capture(f.buffer_mut());
            sel.paint(f.buffer_mut());
        });
        mouse::write_links(&links);
        // The terminal's own cursor, hidden, stands where you type: an input method's window opens there.
        if let Some((x, y)) = at { let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::MoveTo(x, y)); }
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::EndSynchronizedUpdate);
        // What the draw asked for: herdr asked what it shows of the pane; a new size laid out for once it has lasted.
        if std::mem::take(&mut s.size.ask) { held::ask_shown(&tx, &s.ctx.pane, &asking); }
        if s.size.moved_at > 0 && s.size.moved_at != woke_for { woke_for = s.size.moved_at; held::wake(&tx, (s.size.moved_at + s.ctx.settle_ms + 20 - crate::clock::now()).max(0) as u64); }
        let _ = size;
        // Wait for the next thing to happen, then take in everything that is waiting.
        let first = match rx.recv() { Ok(m) => m, Err(_) => break };
        let mut msgs = vec![first];
        while let Ok(m) = rx.try_recv() { msgs.push(m); }
        for m in msgs {
            let fx: Vec<Effect> = match m {
                Msg::Ev(e) => update(&mut s, e),
                Msg::Mouse(me) => match sel.mouse(&me, crate::clock::now()) { mouse::Did::Event(e) => update(&mut s, e), mouse::Did::Copied(n) => { s.flash(format!("Copied {n} character{}.", if n == 1 { "" } else { "s" }), crate::shown::ink::Ink::Muted); vec![] } mouse::Did::Nothing => vec![] },
                Msg::Resize => vec![],
                Msg::Close => vec![Effect::Save, Effect::Close],
                Msg::Load => vec![Effect::Load],
            };
            for e in fx {
                match e {
                    Effect::Run(j) => spawn_job(&tx, j),
                    Effect::Load => run.kick(),
                    Effect::Save => prefs::save(&s),
                    Effect::Poll(now) => poll(&mut s, &mut run, now),
                    Effect::Bell => { use std::io::Write; let _ = std::io::stdout().write_all(b"\x07"); }
                    Effect::Osc(bytes) => { use std::io::Write; let _ = std::io::stdout().write_all(bytes.as_bytes()); }
                    Effect::Close => close = true,
                    Effect::Reload => reload = true,
                    Effect::Take { cols, rows } => held::take(&tx, &s.ctx.pane, cols, rows),
                    Effect::LookWebUi => { let tx = tx.clone(); std::thread::spawn(move || { let urls = crate::webui::command::offer(); if !urls.is_empty() { let _ = tx.send(Msg::Ev(Event::WebUiOffer(urls))); } }); }
                    Effect::Attach { command, args } => {
                        term::leave(false);
                        print!("\x1b[2J\x1b[H");
                        let _ = std::process::Command::new(&command).args(&args).status();
                        let _ = term::enter();
                        let _ = terminal.clear();
                        run.kick();
                        if s.all() { poll(&mut s, &mut run, true); }
                    }
                }
            }
        }
        // What the loader reads closer: the selected task, the one the chat talks to, and your settings while their page is open.
        { let mut w = wants.lock().unwrap(); let (ids, coord, mine, skills) = (vec![s.sel.clone(), s.reply_id.clone()], crate::board::update::coords::wanted(&s), crate::board::update::edit::wanted(&s), crate::board::update::skills::wanted(&s)); if w.ids != ids || w.cwd != s.ctx.cwd || w.coord != coord || w.mine != mine || w.skills != skills { w.ids = ids; w.cwd = s.ctx.cwd.clone(); w.coord = coord; w.mine = mine; w.skills = skills; drop(w); run.kick(); } }
        if STOP.load(Ordering::SeqCst) { prefs::save(&s); close = true; }
        if close || reload { break; }
    }
    prefs::save(&s);
    place::depart();
    if reload {
        term::leave(true);
        start::reload();
        term::leave(false);
        return 1;
    }
    term::leave(false);
    place::restore_board_tab(named);
    if !s.osc_sent.is_empty() { use std::io::Write; let _ = std::io::stdout().write_all(s.osc_sent.keys().map(|id| crate::board::update::beat::osc_clear(id)).collect::<String>().as_bytes()); }
    0
}
