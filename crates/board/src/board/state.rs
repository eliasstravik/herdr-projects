//! The board's state: one plain record of everything it shows and everything you have open, typed or picked. Only
//! update/ changes it (and the drawer, for what a draw measures: where the list scrolled to, how tall the panel is).
//! Nothing here reads a file or asks herdr: what was read comes in as data (data.rs).
use super::data::{Data, Remote};
use super::textbox::TextBox;
use crate::json::*;
use crate::shown::ink::Ink;
use std::collections::{BTreeMap, HashMap};
/// A task as a row of the board, shared: the list, its groups and its rows hold the same one, never a copy.
pub type Rec = std::sync::Arc<Obj>;

/// What the text box is for: a message or a new task, or the words of find.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { New, Find }
/// Where the board runs, and what it was opened with. Fixed while it runs, but for the folder a draft brings it back to.
#[derive(Clone, Debug, Default)]
pub struct Ctx {
    pub cwd: String, pub popup: bool, pub from: String, pub pane: String, pub own_pane: String, pub board_key: String,
    pub own_key: Option<(Vec<String>, String)>, pub can_reload: bool, pub web: bool, pub leave_ms: i64, pub settle_ms: i64, pub in_herdr: bool,
}
/// What `yes` on a question does, and when the question is still about what it names.
#[derive(Clone, Debug, PartialEq)]
pub enum Valid { Always, Mark(String, String) }
#[derive(Clone, Debug)]
pub struct Ask {
    pub q: String, pub yes: super::job::Job, pub no: Option<super::job::Job>, pub valid: Valid, pub keys: Vec<(&'static str, String)>,
    pub detail: String, pub danger: bool, pub nudge: bool,
}
/// The ? menu: which line is picked (-1: none), and how far it is scrolled.
#[derive(Clone, Debug, Default)]
pub struct Menu { pub at: i64, pub top: usize, pub over: i64, pub follow: bool, pub max_top: usize }
/// Words put away, and whom they were for ("" the Coordinator) and where they were typed.
#[derive(Clone, Debug, Default)]
pub struct Away { pub text: String, pub to: String, pub cwd: String, pub warned: bool }
/// What the archive view put aside, to come back to.
#[derive(Clone, Debug, Default)]
pub struct Back { pub sel: String, pub top: usize, pub filter: String, pub mode: Option<Mode>, pub value: String }
/// What the tab not shown had (update/pinned.rs): its grouping, its filter, its folded headings and its selection,
/// to come back to. The Pinned tab is always by machine and project, with no filter.
#[derive(Clone, Debug, Default)]
pub struct TabBack { pub by_repo: bool, pub by_machine: bool, pub show: usize, pub folded: std::collections::BTreeSet<String>, pub sel: String }
/// Whose settings the page shows: a task's, an automation's, yours (what every task starts from, as `set` changes
/// them), your entry for a repo (`set --repo`), or a coordinator's (its key, "main", "machine", "project:<repo>"),
/// with what its heading's page showed before ("you", "repo:<repo>", "" nothing more).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Whose { #[default] Task, Auto, You, Repo(String), Coord(String, String) }
impl Whose {
    /// Yours, your entry for a repo or a coordinator's: no task's or automation's.
    pub fn mine(&self) -> bool { matches!(self, Whose::You | Whose::Repo(_) | Whose::Coord(..)) }
    /// The repo of your entry for a repo ("" for every other).
    pub fn repo(&self) -> &str { if let Whose::Repo(r) = self { r } else { "" } }
    /// What the loader is asked to read for the page (data.rs: mine).
    pub fn wanted(&self) -> String { match self { Whose::Repo(r) => r.clone(), Whose::Coord(k, then) => format!("coord:{k}\t{then}"), _ => String::new() } }
    /// Under a coordinator's own rows, the place its heading's page showed before.
    pub fn then(&self) -> Whose { match self { Whose::Coord(_, t) if t == "you" => Whose::You, Whose::Coord(_, t) => t.strip_prefix("repo:").map(|r| Whose::Repo(r.into())).unwrap_or(Whose::Task), w => w.clone() } }
}
/// The settings page: whose (a task, an automation's row, or a heading for yours), the picked row, and the setting
/// being typed for.
#[derive(Clone, Debug, Default)]
pub struct Editing { pub id: String, pub at: usize, pub typing: String, pub whose: Whose }
/// The first run's one screen.
#[derive(Clone, Debug, Default)]
pub struct Setup { pub file: String, pub typing: bool, pub key: Option<(bool, String, String)>, pub sidebar: Option<bool> }
/// What you sent a task's agent or a coordinator, from the moment you sent it until its own conversation shows it or it
/// says something new: `over` what it had said then, `at` when; `before` the words sent it just before these, maybe still
/// on their way to another machine.
#[derive(Clone, Debug, Default)]
pub struct Sent { pub text: String, pub over: (String, Value, Value), pub at: i64, pub before: Vec<String> }
/// A chat message longer than its box: where you scrolled to, in which chat (`key`). Opened (`top`), it stays open while
/// its conversation changes: `lines` its lines as last drawn, to find the place again, `end` whether it showed the end.
#[derive(Clone, Debug, Default)]
pub struct ChatAt { pub key: String, pub top: Option<usize>, pub lines: Vec<String>, pub end: bool }
#[derive(Clone, Debug, Default)]
pub struct ChatFit { pub total: usize, pub rows: usize, pub top: usize, pub above: usize, pub below: usize, pub latest: usize }
/// One row of the folded list: a heading, a task, a note ("↓ 3 done"), or an automation. A heading is a machine's
/// (`machine`, with every machine shown) or a group's (a repo, a status, a machine's automations: `autos`), selectable
/// by its `id` (update/heads.rs). `folded` holds how many rows it hides, tasks and automations (0: open), `folded_autos`
/// how many of those are automations. `empty`: a machine's with nothing in view, or a repo's used lately with no task in view, which shows folded (`0 tasks`).
#[derive(Clone, Debug)]
pub enum Row { Head { name: String, autos: bool, id: String, machine: bool, folded: usize, folded_autos: usize, empty: bool }, Rec(Rec), Note(String), Auto(Obj) }
impl Row {
    /// A heading that shows folded: one that hides rows, or an empty machine's or repo's.
    pub fn shut(&self) -> bool { matches!(self, Row::Head { folded, empty, .. } if *folded > 0 || *empty) }
}
/// The selection's row in the list as last laid out, to keep the selection steady when rows move: `group` is the
/// heading it stands under (a heading's own id; update/landing.rs for the rows that are no task's).
#[derive(Clone, Debug, Default)]
pub struct SelRow { pub id: String, pub order: String, pub at: i64, pub done: bool, pub base: i64, pub group: String }
/// What a key started on a row, shown on it until what it did is read (update/pending.rs): its word ("Stopping…") and
/// what it says if it fails ("Could not stop"), when, another machine's row as it was then (`was`), whether a read
/// must show it (`wait`), and how it went: `answered` the reads started when its job ended, `failed` why it did not
/// work. `polls`: another machine's answers taken in since the job ended. `kind`: which job it is (Job::kind), the only
/// one whose answer counts, `out` how many of that kind are under way, `through` words or a pane being opened.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pending { pub word: String, pub fail: String, pub at: i64, pub was: String, pub far: bool, pub wait: bool, pub answered: Option<u64>, pub answered_at: i64, pub failed: Option<String>, pub polls: u32, pub kind: String, pub out: u32, pub through: bool }
/// A size somebody else holds the board's terminal at (held.rs), as last seen.
#[derive(Clone, Debug, Default)]
pub struct Held { pub columns: u16, pub rows: u16, pub room: (u16, u16), pub by: Vec<(i64, i64)>, pub known: bool }

/// The web board's own (webui/web.rs): the agent page to go to (`going`: one held, with when, until the web UI shows
/// that pane: webui/going.rs), how often the settings part was asked for, the messages
/// counted (a page shows each once), and the web UI's PCs with the one its page is on. `own_scope`: whose board it
/// shows was picked on the web, and is kept as the web's own; else it is the terminal board's (web.rs).
#[derive(Clone, Debug, Default)]
pub struct Web { pub go: Option<Value>, pub going: Option<(Value, i64)>, pub show_settings: i64, pub msg_n: i64, pub pcs: Option<Vec<crate::webui::pcs::Pc>>, pub at: Option<String>, pub stranger: String, pub stranger_tried: i64, pub followed: Option<String>, pub own_scope: bool }

pub struct State {
    pub ctx: Ctx,
    pub web: Web,
    pub data: Data,
    pub remotes: BTreeMap<String, Remote>,
    /// When herdr last said which machines there are (None: not yet), and whether the scope key is waiting for that
    /// answer (with no other machine known it asks first, and shows every machine only once there is one).
    pub listed_at: Option<i64>, pub seeking: bool,
    /// The machines ("" this one) in the order of herdr's sidebar (remote/sidebar.rs), the other machine whose sidebar
    /// was followed last (kept with the view), and herdr's own list here (None: named by hand in board.machines); the
    /// name that sidebar shows each machine by (none for one it does not show).
    pub order: Vec<String>, pub sidebar_from: Option<String>, pub own_order: Option<Vec<String>>, pub sidebar_names: BTreeMap<String, String>,
    /// The tasks of whoever's board shows, as rows (shown/states.rs: as_row), and what the list makes of them.
    pub items: Vec<Rec>, pub rows: Vec<Row>, pub grouped: Vec<(String, Vec<Rec>)>, pub whole: bool, pub autos: Vec<Obj>, pub jobs: Vec<Obj>, pub steps: Vec<String>,
    pub selection_rows: Vec<SelRow>, pub selection_layout: String,
    pub sel: String, pub top: usize, pub stick: String, pub top_free: bool,
    pub all_done: bool, pub archive_view: bool, pub archive_back: Option<Back>, pub archive_undo: Option<Obj>,
    /// The Pinned tab shows (update/pinned.rs), and what the other tab had; how many rows need you in each tab (the
    /// board's, the Pinned tab's), for the tab names in the header.
    pub pinned_view: bool, pub tab_back: TabBack, pub tab_needs: (usize, usize),
    /// Who the chat talks to: the selected task's agent after space (`reply`), the Coordinator otherwise.
    pub reply: bool, pub reply_id: String,
    pub side: bool, pub full: bool, pub panel_top: usize, pub panel_height: usize, pub panel_length: usize, pub width: usize,
    pub mode: Mode, pub menu: Option<Menu>, pub ask: Option<Ask>, pub by_repo: bool, pub draft_away: Option<Away>,
    pub auto_open: bool, pub auto_at: i64,
    /// The headings folded to their one line, by id (update/heads.rs): kept with the view when the board closes.
    pub folded: std::collections::BTreeSet<String>,
    /// Whose board you see: "" this machine, "*" every machine, or one machine by name.
    pub scope: String,
    /// The filter row on top (update/shows.rs): which one shows, by its place (0 all); where each was drawn, for a
    /// click (line, first and last cell, which); grouped by machine, the third way alt+s groups the list.
    pub show: usize, pub show_n: Vec<usize>, pub show_hits: Vec<(usize, usize, usize, usize)>, pub by_machine: bool,
    pub filter: String, pub msg: String, pub msg_kind: Ink,
    pub chat_open: bool, pub chat_folded: bool, pub just_sent: HashMap<String, Sent>, pub chat_at: Option<ChatAt>, pub chat_fit: Option<ChatFit>, pub chat_rows: Option<(usize, usize)>,
    pub input: TextBox,
    /// The skill list over the text box (update/skills.rs): the one picked, for the draft it was picked in, and the
    /// draft esc closed the list on.
    pub skill_at: (String, usize), pub skill_closed: String,
    pub editing: Option<Editing>, pub edit_line: usize,
    pub setup: Option<Setup>, pub paste_text: String,
    /// The paste screen's first line shown, how far down it can go and how many lines it shows (set as it is drawn).
    pub paste_at: usize, pub paste_more: usize, pub paste_room: usize,
    /// The first ctrl+c (or the last esc) shows how the board closes: until then (ms), and whether esc showed it.
    pub closing: Option<(i64, bool)>, pub leaving: bool, pub leave_at: i64,
    pub attempt_at: HashMap<String, String>,
    /// What a key started and the board has not yet read the result of, by row id (update/pending.rs).
    pub pending: HashMap<String, Pending>,
    pub list_room: usize, pub list_tall: bool, pub list_width: usize, pub fold_sel: String, pub unread_top: usize, pub unread_max: usize,
    pub list_at: Option<(usize, usize)>, pub list_ids: Vec<String>,
    pub told: i64, pub rang: i64, pub err_hid: String, pub told_reply: String, pub lone_at: i64, pub prefixed: i64,
    /// The plugin changed under the board: since when (the stamp it changed to).
    pub stale: String,
    /// A task that just left the list: once it is read again, the message says which task the selection went to.
    pub left: String,
    pub held: Option<Held>, pub guest: bool, pub viewed: bool, pub off_note: String,
    /// What herdr shows of the pane, a viewer holding the terminal, a size that changed for a moment (update/size.rs).
    pub size: super::update::size::Size,
    /// What the last draw put where: the chat's key on the hint line while the message is hidden.
    pub chat_key_shown: bool,
    /// The board looked once whether to ask about showing itself in the herdr web UI (update/web_ui.rs).
    pub web_ui_asked: bool,
    /// The list was too narrow for the numbers' column at the last draw: an automation says on or off in its words.
    pub tags_gone: bool,
    pub now: i64, pub ticks: u64,
    /// The tasks as last told to the terminal (OSC 7501), by id.
    pub osc_sent: HashMap<String, String>,
    /// A call this board rang: its id and when (update/call.rs); the last switch asked by voice that this board made,
    /// and on which call (each call counts its switches from one).
    pub call_link: Option<(String, i64)>, pub call_switch: (String, i64),
}
impl State {
    pub fn new(ctx: Ctx, data: Data) -> State {
        State {
            ctx, web: Web::default(), data, remotes: BTreeMap::new(), listed_at: None, seeking: false, order: vec![], sidebar_from: None, own_order: None, sidebar_names: BTreeMap::new(), items: vec![], rows: vec![], grouped: vec![], whole: false, autos: vec![], jobs: vec![], steps: vec![],
            selection_rows: vec![], selection_layout: String::new(), sel: String::new(), top: 0, stick: String::new(), top_free: false,
            all_done: false, archive_view: false, archive_back: None, archive_undo: None, pinned_view: false, tab_back: TabBack::default(), tab_needs: (0, 0), reply: false, reply_id: String::new(),
            side: false, full: false, panel_top: 0, panel_height: 1, panel_length: 0, width: 120,
            mode: Mode::New, menu: None, ask: None, by_repo: false, draft_away: None, auto_open: false, auto_at: -1, folded: Default::default(), scope: String::new(), show: 0, show_n: vec![], show_hits: vec![], by_machine: false,
            filter: String::new(), msg: String::new(), msg_kind: Ink::Muted, chat_open: true, chat_folded: false, just_sent: HashMap::new(), chat_at: None, chat_fit: None, chat_rows: None,
            input: TextBox::default(), skill_at: (String::new(), 0), skill_closed: String::new(), editing: None, edit_line: 0, setup: None, paste_text: String::new(), paste_at: 0, paste_more: 0, paste_room: 1, closing: None, leaving: false, leave_at: 0,
            attempt_at: HashMap::new(), pending: HashMap::new(), list_room: 0, list_tall: false, list_width: 0, fold_sel: String::new(), unread_top: 0, unread_max: 0, list_at: None, list_ids: vec![],
            told: 0, rang: 0, err_hid: String::new(), told_reply: String::new(), lone_at: 0, prefixed: 0, stale: String::new(), left: String::new(),
            held: None, guest: false, viewed: false, off_note: String::new(), size: Default::default(), chat_key_shown: false, web_ui_asked: false, tags_gone: false, now: crate::clock::now(), ticks: 0, osc_sent: HashMap::new(), call_link: None, call_switch: (String::new(), 0),
        }
    }
    /// The selected task, when a task is selected.
    pub fn current(&self) -> Option<&Obj> { self.items.iter().find(|r| r.s("id") == self.sel).map(|r| &**r) }
    /// The selected words on their way to another machine, when those are selected.
    pub fn job(&self) -> Option<&Obj> { self.jobs.iter().find(|j| j.s("id") == self.sel) }
    /// The selected automation, when the selected row is one.
    pub fn auto(&self) -> Option<&Obj> { self.autos.iter().find(|a| a.s("row") == self.sel) }
    pub fn value(&self) -> String { self.input.text() }
    pub fn all(&self) -> bool { !self.scope.is_empty() }
    /// A message stays until the next key press; an error (red) stays until esc dismisses it.
    pub fn flash(&mut self, text: impl Into<String>, kind: Ink) {
        // Never a green confirmation under the chat: what was done shows where it was done.
        if kind == Ink::Green { return; }
        let said = crate::text::clean(&text.into());
        let said = if crate::re!(r"herdr-projects \w").is_match(&said) { said } else { crate::store::keys::tag_ids(&said, &self.data.repos, &self.data.keys) };
        self.msg = said; self.msg_kind = kind; self.web.msg_n += 1;
    }
    /// Nothing of yours is open or half typed: the board can be swapped for its new version without losing anything.
    pub fn at_rest(&self) -> bool { self.value().is_empty() && self.ask.is_none() && self.menu.is_none() && self.setup.is_none() && self.paste_text.is_empty() && self.mode == Mode::New }
    /// With nothing of a task's open, your words go to a Coordinator: this machine's, or the one machine you look at.
    pub fn talks_to_coord(&self) -> bool { self.data.can_dispatch || (!self.scope.is_empty() && self.scope != "*" && self.remotes.contains_key(&self.scope)) }
}
