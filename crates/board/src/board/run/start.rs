//! The board starting: herdr's theme (the terminal is asked for its own colours first), the keeper started, the how-to
//! and herdr's config blocks brought up to date, the view it had when it last closed, and a first look at the tasks
//! from disk alone, so the first frame is up at once; what herdr says of them follows a moment later. Then the threads
//! that feed the loop. And a board loading its new version in the same terminal.
use super::{place, prefs, term, Msg};
use crate::board::data::{Data, Wants};
use crate::board::state::{Ctx, Setup, State};
use crate::board::theme::Theme;
use crate::board::update::{list::relist, Event};
use crate::herdr::cli::{current_socket, in_herdr};
use crate::json::*;
use crate::shown::ink::Ink;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex};

fn env(k: &str) -> String { std::env::var(k).unwrap_or_default() }
/// What the board shows before herdr was asked: the tasks as they were saved.
pub fn quick_data(cwd: &str) -> Data {
    let tasks = crate::store::tasks::list(crate::store::tasks::ListOpts { include_archived: true, ..Default::default() });
    let row = |t: &crate::store::tasks::Task| { let mut x = crate::flow::live::saved(t); x.put("research", crate::research::is_research(t)); x.put("chatOnly", crate::chat_only::is_chat(t)); std::sync::Arc::new(crate::shown::states::as_row(&x)) };
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).clamp(1, 8);
    let items = if tasks.len() < 32 { tasks.iter().map(row).collect() } else { let chunk = tasks.len().div_ceil(n); std::thread::scope(|sc| tasks.chunks(chunk).map(|c| sc.spawn(move || c.iter().map(row).collect::<Vec<_>>())).collect::<Vec<_>>().into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()) };
    let roots: std::collections::BTreeMap<String, String> = crate::store::repos::known_repos().iter().map(|(k, v)| (k.clone(), text(v))).collect();
    let here = crate::board::data::repo_here(cwd);
    let now = crate::clock::now();
    Data { items, autos: crate::store::automations::automations().into_iter().map(|a| crate::board::data::shown_auto(a, now)).collect(), keys: crate::store::keys::repo_keys(), repos: roots.keys().cloned().collect(), roots,
        session: crate::herdr::cli::session_name(&current_socket()), repo: here.0, repo_root: here.1, can_dispatch: true, ..Default::default() }
}
pub fn prepare(folder: &std::path::Path) -> (State, Theme, Option<(String, String)>) {
    crate::runner::user_env(); // the popup starts with herdr's server environment: helpers and git need the user's PATH
    // herdr's theme. The terminal is asked for its own text and background colours (the greys are made readable on
    // them), and answers while the tasks are read.
    let asked = term::ask_colours();
    crate::keeper::ensure_keeper(); // the board opening is one of the keeper's starters
    crate::flow::send::shim(); // the launcher in the data folder, there from the first time the board opens
    crate::leftovers::sweep::at_start(); // what ended tasks left behind (docs/disk-space.md)
    let _ = crate::extras::sync_skill(); crate::extras::remove_popups(); crate::extras::refresh_sidebar();
    let reloads: i64 = env("HERDR_PROJECTS_RELOADS").parse().unwrap_or(0);
    let named = place::name_board_tab(&place::board_pane());
    let popup = place::in_popup();
    let ctx = Ctx { cwd: folder.display().to_string(), popup, from: place::context().0, pane: if env("HERDR_ENV") == "1" && env("TMUX").is_empty() { env("HERDR_PANE_ID") } else { String::new() },
        own_pane: place::own_name(), board_key: crate::extras::board_key(), own_key: crate::extras::board_presses(), can_reload: reloads < 5, web: false, leave_ms: 700, settle_ms: 1000, in_herdr: in_herdr() };
    // Every repo has a short key: given before the first look, and the ones given since the board last opened said once.
    let keys = crate::store::keys_given::keys_note();
    let mut s = view(ctx, folder);
    // The one-time move of an older version's folders, said once.
    let (home, envn) = (crate::store::files::user_home(), crate::store::move_home::env_now());
    let moved = crate::store::move_home::move_note(&home, &envn);
    if !moved.is_empty() { crate::store::move_home::move_told(&home, &envn); }
    let said: Vec<String> = [moved, keys].into_iter().filter(|x| !x.is_empty()).collect();
    if !said.is_empty() { s.flash(said.join("  "), Ink::Muted); }
    start_setup(&mut s);
    let seen = term::colours_told(asked, 250);
    let theme = Theme::load(seen.light, seen);
    (s, theme, named)
}
/// What the board opens on: the view it had when it last closed (the popup's own) with the top row selected, in the
/// repo a draft left in its box was typed for, on the task of the pane you came from.
pub fn view(mut ctx: Ctx, folder: &std::path::Path) -> State {
    let p = prefs::read(ctx.popup);
    // Opened from the Coordinator's own pane: its folder is nobody's repo. The board is where it was when you left it.
    if crate::coordinator::requests::in_home(folder, &current_socket()) { ctx.cwd = if p.b("cwd") && std::path::Path::new(p.s("cwd")).exists() { p.t("cwd") } else { crate::store::files::user_home().display().to_string() }; }
    let data = quick_data(&ctx.cwd);
    let mut s = State::new(ctx, data);
    prefs::apply(&mut s, &p);
    // It opens on the top row (All machines, or the one machine shown), so the chat starts with its coordinator.
    s.sel = crate::board::update::top::top_id(&s);
    // Words left in the box when it closed were typed for the repo it was in then: the board stays there.
    if !p.s("draft").trim().is_empty() { crate::board::update::setup::to_repo_of(&mut s, p.s("cwd")); }
    // The popup opens on the task that belongs to where you are: the one whose agent runs in the pane you came from.
    if s.ctx.popup && !s.ctx.from.is_empty() { if let Some(t) = place::task_at(&s.ctx.from).filter(|t| !t.b("archivedAt")) { s.sel = t.t("id"); s.stick = t.t("id"); if !s.scope.is_empty() && s.scope != "*" { s.scope.clear(); } } }
    relist(&mut s);
    s
}
/// The first run: one screen asks what the plugin may add to herdr's settings. Only inside herdr, only what is missing.
fn start_setup(s: &mut State) {
    if (!in_herdr() && env("HERDR_PLUGIN_ID").is_empty()) || crate::runner::inline() { return; }
    let o = crate::extras::offers();
    if o.b("asked") { return; }
    if o.b("sidebarKept") { s.paste_text = o.t("sidebarKept"); } // the user's own rows stay: said once, ours to paste
    if o.at("key").is_null() && o.at("sidebar").is_null() { crate::extras::answer(json!({})); return; }
    let key = o.at("key");
    s.setup = Some(Setup { file: o.t("file"), typing: false, key: if key.is_object() { Some((key.b("suggest"), key.t("suggest"), key.t("note"))) } else { None }, sidebar: if o.at("sidebar").is_null() { None } else { Some(true) } });
}
/// The threads that feed the loop: keys and the mouse, the beat, the loader, the program's own stamp, the terminal's size.
pub fn threads(tx: &Sender<Msg>, wants: &Arc<Mutex<Wants>>, load: &Arc<(Mutex<bool>, Condvar)>) {
    use crossterm::event::{read, Event as CE};
    let t = tx.clone();
    std::thread::spawn(move || loop {
        let Ok(e) = read() else { break };
        let m = match e {
            CE::Key(k) => match super::input::press_of(&k) { Some(p) => Msg::Ev(Event::Key(p)), None => continue },
            CE::Paste(text) => Msg::Ev(Event::Paste(text)),
            CE::Mouse(m) => Msg::Mouse(m),
            CE::Resize(..) => Msg::Resize,
            _ => continue,
        };
        if t.send(m).is_err() { break; }
    });
    let t = tx.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        if super::STOP.load(std::sync::atomic::Ordering::SeqCst) { let _ = t.send(Msg::Close); }
        if t.send(Msg::Ev(Event::Tick(crate::clock::now()))).is_err() { break; }
    });
    let (t, w, l) = (tx.clone(), wants.clone(), load.clone());
    std::thread::spawn(move || loop {
        { let (m, c) = &*l; let g = m.lock().unwrap(); let (mut g, _) = c.wait_timeout_while(g, std::time::Duration::from_millis(1000), |go| !*go).unwrap(); *g = false; }
        // With a keeper alive the board only looks; without one it does the keeper's tick itself.
        if !crate::keeper::keeper_alive() { crate::keeper::tick_once(&current_socket(), true); }
        let wants = w.lock().unwrap().clone();
        if t.send(Msg::Ev(Event::Loaded(Box::new(crate::board::data::load(&wants))))).is_err() { break; }
    });
    let t = tx.clone();
    std::thread::spawn(move || {
        let exe = crate::store::files::bin();
        let first = crate::keeper::code_stamp(&exe);
        // Every five seconds (HERDR_PROJECTS_STAMP_MS: the tests that load a new version look sooner).
        let every = env("HERDR_PROJECTS_STAMP_MS").parse().unwrap_or(5000u64);
        loop {
            std::thread::sleep(std::time::Duration::from_millis(every));
            let now = crate::keeper::code_stamp(&exe);
            if t.send(Msg::Ev(Event::Stamp(if now == first { String::new() } else { now }))).is_err() { break; }
        }
    });
    if !env("HERDR_PANE_ID").is_empty() && env("HERDR_ENV") == "1" && env("TMUX").is_empty() {
        let t = tx.clone();
        // A look at once and every five seconds: what holds the size is said once two looks agree (update/beat.rs).
        std::thread::spawn(move || loop {
            let (c, r) = crossterm::terminal::size().unwrap_or((0, 0));
            if t.send(Msg::Ev(Event::Held(super::held::look(c, r), super::held::viewed()))).is_err() { break; }
            std::thread::sleep(std::time::Duration::from_millis(5000));
        });
    }
    // herdr keeps its popup with the tab it opened over: this board closes when another tab shows.
    if place::in_popup() { let t = tx.clone(); watch_tab(None, &place::context().1, move || { let _ = t.send(Msg::Close); }); }
}
/// `gone` when herdr shows a tab other than `tab`, or, once herdr tells, when it says `tab` no longer shows (it changed
/// while the board was starting). A board not told its tab watches nothing.
pub fn watch_tab(socket: Option<String>, tab: &str, gone: impl Fn() + Send + Sync + 'static) {
    if tab.is_empty() { return; }
    let gone = Arc::new(gone);
    let (g1, g2, tab1, tab2, at) = (gone.clone(), gone, tab.to_string(), tab.to_string(), socket.clone());
    crate::herdr::socket::on_tab_shown(socket, move |shown| { if shown != tab1 { g1(); } }, move || {
        if let Ok(r) = crate::herdr::socket::ask("tab.get", &json!({ "tab_id": tab2 }), at.as_deref()) { if r.at("tab").at("focused") == &json!(false) { g2(); } }
    });
}
/// The program changed under the open board: the same command starts again on the new program, in this terminal,
/// over the old frame. Returns only when that could not be done.
pub fn reload() {
    use std::os::unix::process::CommandExt;
    let reloads: i64 = env("HERDR_PROJECTS_RELOADS").parse().unwrap_or(0);
    let exe = crate::store::files::bin();
    let err = std::process::Command::new(exe).args(std::env::args_os().skip(1)).env(term::KEPT, "1").env("HERDR_PROJECTS_RELOADS", (reloads + 1).to_string()).exec();
    eprintln!("herdr-projects could not start its new version: {err}");
}
