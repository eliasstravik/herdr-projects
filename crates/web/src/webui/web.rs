//! The board with no terminal: the same state and update the terminal board runs (board/), one for each browser that
//! has the board open, so each has its own selection, question and message. Nothing is drawn here: the page asks for
//! the board as data (`board`), and a button presses the key it stands for (view.rs). A job runs as the terminal
//! board's does; one that asks another machine answers into the board later, and the page sees it on its next look.
//! One nobody asked for in ten minutes is let go.
use crate::board::data::{load, Data, Wants};
use crate::board::job;
use crate::board::state::{Ctx, State};
use crate::board::theme::Theme;
use crate::board::update::{update, Effect, Event};
use crate::json::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// `boards_at`: when the sidebar's Boards section last had the other machines asked (boards.rs).
/// `id`: the page's own name for its board (a call it holds is called by it: voice/board.rs).
pub struct Client { pub s: State, pub inbox: Arc<Mutex<Vec<Event>>>, pub at: i64, pub machines_at: i64, pub boards_at: i64, pub id: String }
const IDLE: i64 = 10 * 60000;
/// How long a call waits for what a button started (a merge, an agent started); longer, it answers on a later look.
const WAIT: u64 = 25;
static CLIENTS: Mutex<Option<HashMap<String, Arc<Mutex<Client>>>>> = Mutex::new(None);
pub fn theme() -> &'static Theme { static T: std::sync::OnceLock<Theme> = std::sync::OnceLock::new(); T.get_or_init(Theme::web) }
fn prefs_file() -> std::path::PathBuf { crate::store::files::data_dir().join("web-view.json") }
/// What the terminal board last kept of its view (view.json): the web board reads it and never writes it.
fn terminal_view() -> crate::json::Obj { crate::store::files::read_obj(&crate::store::files::data_dir().join("view.json")).unwrap_or_default() }
/// Where the web board is "in": the repo the terminal board was last opened in, else the home folder.
fn last_folder() -> String {
    let cwd = terminal_view().t("cwd");
    if !cwd.is_empty() && std::path::Path::new(&cwd).exists() { cwd } else { crate::store::files::user_home().display().to_string() }
}
fn open() -> Client {
    let ctx = Ctx { cwd: last_folder(), web: true, ..Default::default() };
    let keys = crate::store::keys_given::keys_note(); // every repo has a short key: what was given, said once
    let mut s = State::new(ctx, Data::default());
    if !keys.is_empty() { s.flash(keys, crate::shown::ink::Ink::Muted); }
    // The web board keeps its own few choices, in a file of its own: it never writes the terminal board's.
    // The selection is where the terminal board left it, and so is whose board it shows (this machine, every machine
    // or one), unless a board was picked on the web: that one is the web's own ("scope", kept before, is not: every
    // web board kept "this machine" by itself, and showed nothing while the tasks were on the other machines).
    let view = terminal_view();
    s.sel = view.t("sel");
    let p = crate::store::files::read_obj(&prefs_file()).unwrap_or_default();
    s.by_repo = p.b("byRepo"); s.by_machine = p.b("byMachine") && !s.by_repo; s.all_done = p.b("allDone"); s.chat_open = p.s("chat") != "closed";
    let own = p.get("ownScope").and_then(|x| x.as_str()).map(String::from);
    s.web.own_scope = own.is_some();
    s.scope = own.unwrap_or_else(|| view.t("scope"));
    // The filter shown on the filter row, the web's own as the terminal board keeps its own.
    s.show = p.at("show").as_u64().map(|n| (n as usize).min(crate::board::update::shows::SHOWS.len() - 1)).unwrap_or(0);
    s.folded = p.arr("folded").iter().filter_map(|x| x.as_str()).map(String::from).collect();
    Client { s, inbox: Arc::new(Mutex::new(vec![])), at: 0, machines_at: 0, boards_at: 0, id: String::new() }
}
pub fn client(id: &str) -> Arc<Mutex<Client>> {
    let key: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').take(40).collect();
    let key = if key.is_empty() { "one".to_string() } else { key };
    let now = crate::clock::now();
    let (c, fresh) = {
        let mut all = CLIENTS.lock().unwrap();
        let map = all.get_or_insert_with(HashMap::new);
        map.retain(|k, c| *k == key || c.try_lock().map(|c| now - c.at <= IDLE).unwrap_or(true));
        let fresh = !map.contains_key(&key);
        (map.entry(key.clone()).or_insert_with(|| Arc::new(Mutex::new(Client { id: key, ..open() }))).clone(), fresh)
    };
    { let mut g = c.lock().unwrap(); g.at = now; if fresh { reload(&mut g); if g.s.all() { poll(&mut g, true); } } }
    c
}
/// Whose board was picked on the web (the sidebar's Boards, the scope key): the web's own from now on, unless it is
/// the terminal board's own choice, which the web then follows again.
fn picked(c: &mut Client) {
    c.s.web.own_scope = c.s.scope != terminal_view().t("scope");
    save_prefs(&c.s);
}
pub fn save_prefs(s: &State) {
    // While it shows the machine the web UI is on (view.rs), what is kept is what it showed before it went there.
    let scope = s.web.followed.clone().unwrap_or_else(|| s.scope.clone());
    // Only a board picked on the web is kept (picked): any other save leaves the web following the terminal board.
    let own = if s.web.own_scope { json!(scope) } else { Value::Null };
    let _ = crate::store::files::write_json(&prefs_file(), &json!({ "byRepo": s.by_repo, "byMachine": s.by_machine, "allDone": s.all_done, "ownScope": own, "folded": s.folded, "show": s.show, "chat": if s.chat_open { "open" } else { "closed" } }));
}
/// Everything read again: the tasks, the closer look at the selected one and the one the chat talks to.
pub fn reload(c: &mut Client) {
    let wants = Wants { ids: vec![c.s.sel.clone(), c.s.reply_id.clone()], cwd: c.s.ctx.cwd.clone(), own_pane: String::new(), web: true, coord: crate::board::update::coords::wanted(&c.s), mine: None, looks: crate::board::update::coords::looks_wanted(&c.s), skills: String::new() };
    let d = load(&wants);
    let fx = update(&mut c.s, Event::Loaded(Box::new(d)));
    effects(c, fx, false);
}
/// Do what the board's update answered. `Load` is done once, after the rest.
pub fn effects(c: &mut Client, fx: Vec<Effect>, mut again: bool) {
    let mut queue: std::collections::VecDeque<Effect> = fx.into();
    while let Some(e) = queue.pop_front() {
        match e {
            // A test answers for herdr in its own thread (herdr::cli::set_fake): there a job runs in that thread too.
            Effect::Run(j) if crate::herdr::cli::fake().is_some() => { let d = job::run(j); queue.extend(update(&mut c.s, Event::Done(d))); }
            Effect::Run(j) => {
                let far = matches!(j, job::Job::Far { .. } | job::Job::Dispatch { .. } | job::Job::WebOpen { far: Some(_), .. });
                let (tx, rx) = std::sync::mpsc::channel();
                let inbox = c.inbox.clone();
                std::thread::spawn(move || { let d = job::run(j); if let Err(e) = tx.send(d) { inbox.lock().unwrap().push(Event::Done(e.0)); } });
                let waited = if far { None } else { rx.recv_timeout(std::time::Duration::from_secs(WAIT)).ok() };
                match waited {
                    Some(d) => queue.extend(update(&mut c.s, Event::Done(d))),
                    None => { let inbox = c.inbox.clone(); std::thread::spawn(move || { if let Ok(d) = rx.recv() { inbox.lock().unwrap().push(Event::Done(d)); } }); }
                }
            }
            Effect::Load => again = true,
            Effect::Save => save_prefs(&c.s),
            Effect::Poll(now) => poll(c, now),
            Effect::Bell | Effect::Osc(_) | Effect::Close | Effect::Reload | Effect::Attach { .. } | Effect::Take { .. } | Effect::LookWebUi => {} // a terminal's own
        }
    }
    if again { reload(c); }
}
/// Other machines are asked for their tasks on threads of their own; their answers come in on the next look.
pub fn poll(c: &mut Client, now_too: bool) {
    let now = crate::clock::now();
    if now - c.machines_at > 60000 { c.machines_at = now; let inbox = c.inbox.clone(); std::thread::spawn(move || { let l = crate::remote::machines(); inbox.lock().unwrap().push(Event::Machines(l)); }); }
    for (label, m) in c.s.remotes.iter_mut() {
        if m.busy || (!now_too && !m.ok && now - m.fail_at < 30000) { continue; }
        m.busy = true;
        let (inbox, mc, label) = (c.inbox.clone(), m.machine.clone(), label.clone());
        std::thread::spawn(move || { let a = crate::remote::ask_list(&mc, 20000); inbox.lock().unwrap().push(Event::Remote { label, answer: a }); });
    }
}
/// What came in since the last call: answers of jobs and of other machines.
pub fn take_in(c: &mut Client) {
    let events: Vec<Event> = std::mem::take(&mut *c.inbox.lock().unwrap());
    for e in events { let fx = update(&mut c.s, e); effects(c, fx, false); }
}
/// Everything the page shows, and a short mark of it: a page that already has this one is told only that.
pub fn snapshot(c: &mut Client, have: &str) -> Value {
    let t = theme();
    let call = super::voice::board::shown(&c.id);
    let s = &mut c.s;
    let r = s.current().cloned().or_else(|| s.job().cloned());
    let a = s.auto().cloned();
    // An automation's row opens the automation, whole, where a task's opens the task.
    let auto_open = a.is_some() && s.auto_open;
    let coord = if r.is_none() && a.is_none() { crate::board::update::pilot::selected(s) } else { None };
    let panel = if auto_open { super::model::panel::auto_panel_of(s, t, a.as_ref().unwrap()) } else if s.full { r.as_ref().map(|r| super::model::panel::panel_of(s, t, r)).or_else(|| coord.as_ref().map(|c| super::model::panel::coord_panel_of(s, c))).unwrap_or(Value::Null) } else { Value::Null };
    // With a task or an automation open: every setting of it, to see and to change.
    let settings = if panel.is_null() || panel.b("coordinator") { Value::Null } else if auto_open { super::model::settings::settings_of(s, a.as_ref().unwrap(), true) } else { r.as_ref().map(|r| super::model::settings::settings_of(s, r, false)).unwrap_or(Value::Null) };
    // `calls`: whether there are voice calls at all (voice.calls); off, the page has no Call button.
    let mut data = obj(json!({ "list": super::model::list::list_of(s, t), "panel": panel, "settings": settings, "call": call, "calls": crate::voice::calls_on() }));
    for (k, v) in super::model::side::side_of(s, t) { data.insert(k, v); }
    let text = serde_json::to_string(&data).unwrap_or_default();
    let mark: String = crate::store::files::base64url(&sha1_smol::Sha1::from(text.as_bytes()).digest().bytes()).chars().take(16).collect();
    if mark == have { return json!({ "same": true, "mark": mark }); }
    data.put("mark", mark);
    Value::Object(data)
}
/// One call of the page. `role`: "drive" may change things, "watch" may only look (door.rs).
pub fn call(client_id: &str, call: &str, args: Value, machine: &str, pcs: Option<Vec<super::pcs::Pc>>) -> Result<Value, String> { call_with(client_id, call, args, machine, pcs, None) }
/// The same, with the web UI as the front door reaches it (a file attached for another machine goes through it).
pub fn call_with(client_id: &str, call: &str, args: Value, machine: &str, pcs: Option<Vec<super::pcs::Pc>>, ui: Option<&dyn super::attach::far::WebUi>) -> Result<Value, String> {
    // From here on the board's words name a key as its button on the page.
    crate::board::keys::name_as_buttons(true);
    let c = client(client_id);
    let mut g = c.lock().unwrap();
    let c = &mut *g;
    take_in(c);
    super::follow::follow(c, pcs, Some(machine));
    let drive = args.s("role") == "drive";
    let watch = || Err::<Value, String>("This device can only watch.".into());
    match call {
        "board" => { let now = crate::clock::now(); let fx = update(&mut c.s, Event::Tick(now)); effects(c, fx, true); Ok(snapshot(c, args.s("have"))) }
        "press" => { let draft = super::view::press(c, args.s("id"), drive)?; if args.s("id") == "scope" { picked(c); } let mut o = snapshot(c, ""); o.as_object_mut().unwrap().insert("draft".into(), json!(draft)); Ok(o) }
        "send" => {
            if !drive { return watch(); }
            let to = args.at("to").as_object().map(|t| (t.t("id"), t.b("reply"), t.get("scope").and_then(|x| x.as_str()).map(|x| x.chars().take(80).collect())));
            let kept = super::view::send(c, args.s("text"), args.b("delegate"), to);
            let mut o = snapshot(c, ""); o.as_object_mut().unwrap().insert("kept".into(), json!(kept)); Ok(o)
        }
        // Where the web UI's own chat of whoever the chat talks to is, to dock it under the list (null: the board's own).
        "dock" => { let d = super::dock::dock(c); let mut o = snapshot(c, ""); o.as_object_mut().unwrap().insert("dock".into(), d); Ok(o) }
        "attach" => { if !drive { return watch(); } super::attach::attach_with(c, &args, ui) }
        "set" => {
            if !drive { return watch(); }
            let v = args.at("value");
            super::view::set_setting(c, args.s("id"), args.s("name"), if v.is_null() { None } else { Some(text(v)) })?;
            Ok(snapshot(c, ""))
        }
        // These only change what this page looks at: its selection, its filter, its folded headings.
        "pick" => { super::view::pick(c, args.s("id"), args.b("open")); Ok(snapshot(c, "")) }
        "find" => { super::view::find(c, args.s("text")); Ok(snapshot(c, "")) }
        "fold" => { super::view::fold(c, args.s("id")); Ok(snapshot(c, "")) }
        // The filter row: a filter shown (its place in it), and the grouping put back to by status (a chip's ×).
        "show" => { super::view::show(c, args.at("n").as_u64().unwrap_or(0) as usize); Ok(snapshot(c, "")) }
        "group" => { super::view::by_status(c); Ok(snapshot(c, "")) }
        // The sidebar's Boards section: every machine's board at a glance, and picking one of them.
        "boards" => Ok(super::boards::boards(c)),
        "scope" => { super::boards::choose(c, args.s("to"))?; picked(c); Ok(snapshot(c, "")) }
        other => Err(format!("Unknown call {other}")),
    }
}
/// Every browser's board let go, and the web UI's PCs as last asked (a test's next world starts with none).
pub fn forget() { *CLIENTS.lock().unwrap() = None; super::pcs::forget(); }
