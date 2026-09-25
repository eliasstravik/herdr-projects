//! Groups Herdr's sidebar by project as a rail: a corner with the project
//! name, each card's own status icon on the rail, and an end corner, the same
//! in the agents and the Spaces list. The rail's colour and weight say the
//! project's state (heavy red: needs you; lilac: working; grey: idle; dashed:
//! no project).
//!
//! Herdr 0.9.1 puts ` · ` after every token but the status icon, and draws a
//! card's row 0 one indent left of its other rows. So the rail never shares a
//! row with Herdr's tokens: every card gets a row 0 of ours (the corner, or a
//! `│` connector), which puts its own row at the continuation indent, right on
//! the rail. Sub-lines and the end corner are rows of ours too. Worktree
//! children are drawn by Herdr with `├─`/`└─` in that same column, so they
//! join the rail, and a block that ends in one is closed by Herdr's `└─`.
//!
//! Agents sort by `$hp_group` in the agent view; Spaces are moved into one
//! block per project with `workspace.move`. Anything in no project forms the
//! `other` block, last.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::herdr::{Agent, CALL_TIMEOUT, Herdr, Workspace};

/// Herdr drops a token whose value is only whitespace; U+2800 (braille blank)
/// is kept and draws as an empty cell.
pub const BLANK: &str = "\u{2800}";
/// Moves a row-0 value to the continuation column (Herdr trims leading
/// whitespace, not U+2800).
const PAD: &str = "\u{2800}\u{2800}";
/// Ends every home Space label. The Space rows hide a `workspace` value that
/// contains it and show `$hp_home` instead, so the home row says `home` under
/// the corner that already names the project. A mark in the label, not a list
/// of names in the config, because a client draws another machine's Spaces
/// with its own config.
pub const HOME_MARK: char = '\u{2800}';
/// The group key of whatever belongs to no project: sorts after every slug.
const OTHER: &str = "~";
/// The rows of a card, in the order the config lists them.
pub const SLOTS: [&str; 4] = ["top", "con", "sub", "end"];

/// A project's state, as its rail shows it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Rail {
    Needs,
    Working,
    #[default]
    Idle,
    /// The `other` block: nothing of any project.
    Other,
}

impl Rail {
    pub const ALL: [Rail; 4] = [Rail::Needs, Rail::Working, Rail::Idle, Rail::Other];

    pub fn suffix(self) -> &'static str {
        match self {
            Rail::Needs => "n",
            Rail::Working => "w",
            Rail::Idle => "i",
            Rail::Other => "o",
        }
    }

    /// Herdr's palette: red is what Herdr uses for blocked, the accent lilac
    /// for active, overlay tones for quiet.
    pub fn color(self) -> &'static str {
        match self {
            Rail::Needs => "#f38ba8",
            Rail::Working => "#cba6f7",
            Rail::Idle => "#7f849c",
            Rail::Other => "#6c7086",
        }
    }

    /// `(corner, rail, end)`.
    fn glyphs(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Rail::Needs => ("┏━", "┃", "┗━"),
            Rail::Other => ("┌╌", "╎", "└╌"),
            Rail::Working | Rail::Idle => ("┌─", "│", "└─"),
        }
    }
}

pub fn token(slot: &str, rail: Rail) -> String {
    format!("hp_{slot}_{}", rail.suffix())
}

/// Every token this module writes.
pub fn tokens() -> Vec<String> {
    let mut all: Vec<String> = SLOTS.iter().flat_map(|slot| Rail::ALL.map(|rail| token(slot, rail))).collect();
    all.extend(["hp_gap", "hp_home", "hp_group"].map(String::from));
    all
}

/// Tokens of the 0.2.17/0.2.18 layout: cleared with a pane's or Space's own
/// tokens, and otherwise left to expire (they had a TTL).
pub const LEGACY: [&str; 7] = ["hp_top", "hp_other", "hp_note", "hp_tail", "hp", "hp_state", "hp_activity"];

/// One agent of a project: its pane, sort key and sub-line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PaneRow {
    pub pane: String,
    pub key: String,
    /// `review · report`, `~40% · Writing tests`, or empty.
    pub sub: String,
}

/// One project's part of a session, as its tick saw it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectPart {
    pub name: String,
    pub rail: Rail,
    /// The project's agents in panel order.
    pub panes: Vec<PaneRow>,
    /// The home Space.
    pub home: String,
    /// The project's Spaces, home first, then each thread's repository Space
    /// and its own.
    pub spaces: Vec<String>,
}

/// Every project's part in one Herdr session, keyed by slug.
pub type Parts = BTreeMap<String, ProjectPart>;

/// `!` sorts before every slug character, so keys sort as their slugs do.
pub fn coordinator_key(slug: &str, pane: &str) -> String {
    format!("{slug}!0!{pane}")
}

pub fn thread_key(slug: &str, rank: u8, id: &str) -> String {
    format!("{slug}!1!{rank}!{id}")
}

/// Tokens for one card: `Some` sets, `None` clears.
pub type Tokens = Vec<(String, Option<String>)>;

#[derive(Debug, Default, PartialEq)]
pub struct Plan {
    pub agents: Vec<(String, Tokens)>,
    pub spaces: Vec<(String, Tokens)>,
    /// The Space order Herdr should end up with.
    pub order: Vec<String>,
}

/// What a card's row 0 shows.
enum Head<'a> {
    /// The first card of a block: the corner with the block's name.
    Corner(&'a str),
    /// Any other card: a piece of rail, so its own row lands on the rail.
    Connector,
    /// A worktree child: Herdr draws its `├─`/`└─` on the rail itself.
    Child,
}

/// One card's rail tokens. Every rail token of every state is listed, so a
/// project that changes state clears the old colour.
fn card(rail: Rail, head: Head, sub: &str, end: bool, gap: bool) -> Tokens {
    let (corner, line, close) = rail.glyphs();
    let mut set: HashMap<&str, String> = HashMap::new();
    match head {
        Head::Corner(name) => {
            set.insert("top", format!("{PAD}{corner} {name}"));
        }
        Head::Connector => {
            set.insert("con", format!("{PAD}{line}"));
        }
        Head::Child => {}
    }
    if !sub.is_empty() {
        set.insert("sub", format!("{line} {sub}"));
    }
    if end {
        set.insert("end", close.to_string());
    }
    let mut out: Tokens = Vec::new();
    for slot in SLOTS {
        for r in Rail::ALL {
            out.push((token(slot, r), if r == rail { set.get(slot).cloned() } else { None }));
        }
    }
    out.push(("hp_gap".into(), gap.then(|| BLANK.to_string())));
    out
}

fn name_and_rail<'a>(parts: &'a Parts, group: &str) -> (&'a str, Rail) {
    match parts.get(group) {
        Some(part) => (part.name.as_str(), part.rail),
        None => ("other", Rail::Other),
    }
}

/// The whole layout of one session. Agents: projects by slug, each in its own
/// key order, then every other agent in Herdr's order. Spaces: the same
/// blocks; a worktree Space Herdr draws under its repository Space stays
/// there.
pub fn plan(parts: &Parts, agents: &[Agent], workspaces: &[Workspace]) -> Plan {
    let mut out = Plan::default();
    if parts.is_empty() {
        return out;
    }

    // Agents.
    let mut owner: HashMap<&str, (&str, &PaneRow)> = HashMap::new();
    for (slug, part) in parts {
        for row in &part.panes {
            owner.entry(row.pane.as_str()).or_insert((slug.as_str(), row));
        }
    }
    let mut rows: Vec<(String, String, &str, &str)> = agents
        .iter()
        .enumerate()
        .map(|(i, a)| match owner.get(a.pane_id.as_str()) {
            Some((slug, row)) => ((*slug).to_string(), row.key.clone(), a.pane_id.as_str(), row.sub.as_str()),
            None => (OTHER.to_string(), format!("{OTHER}!{i:04}"), a.pane_id.as_str(), ""),
        })
        .collect();
    rows.sort_by(|a, b| a.1.cmp(&b.1));
    for (i, (group, key, pane, sub)) in rows.iter().enumerate() {
        let first = i == 0 || rows[i - 1].0 != *group;
        let next = rows.get(i + 1);
        let last = next.is_none_or(|n| n.0 != *group);
        let (name, rail) = name_and_rail(parts, group);
        let head = if first { Head::Corner(name) } else { Head::Connector };
        let mut tokens = card(rail, head, sub, last, last && next.is_some());
        tokens.push(("hp_group".into(), Some(key.clone())));
        out.agents.push((pane.to_string(), tokens));
    }

    // Spaces: a stable sort into blocks.
    let mut space_owner: HashMap<&str, (&str, usize)> = HashMap::new();
    for (slug, part) in parts {
        for (i, id) in part.spaces.iter().enumerate() {
            space_owner.entry(id.as_str()).or_insert((slug.as_str(), i));
        }
    }
    let homes: HashSet<&str> = parts.values().map(|p| p.home.as_str()).filter(|h| !h.is_empty()).collect();
    let mut ordered: Vec<(&str, usize, &Workspace)> = workspaces
        .iter()
        .enumerate()
        .map(|(i, w)| match space_owner.get(w.workspace_id.as_str()) {
            Some((slug, pos)) => (*slug, *pos, w),
            None => (OTHER, i, w),
        })
        .collect();
    ordered.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    out.order = ordered.iter().map(|(_, _, w)| w.workspace_id.clone()).collect();

    // What Herdr draws from that order (ui::sidebar::workspace_list_entries):
    // a repository with two or more Spaces, one of them its main checkout,
    // is one entry at its first member, the main checkout first.
    let key = |w: &Workspace| w.worktree.as_ref().map(|t| t.repo_key.clone()).filter(|k| !k.is_empty());
    let mut members: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, (_, _, w)) in ordered.iter().enumerate() {
        if let Some(k) = key(w) {
            members.entry(k).or_default().push(i);
        }
    }
    let parent_of = |k: &str| -> Option<usize> {
        let list = members.get(k)?;
        if list.len() < 2 {
            return None;
        }
        list.iter().copied().find(|i| ordered[*i].2.worktree.as_ref().is_some_and(|t| !t.is_linked_worktree))
    };
    // (index into `ordered`, drawn indented, block of its top-level entry)
    let mut drawn: Vec<(usize, bool, &str)> = Vec::new();
    let mut emitted = HashSet::new();
    for (i, (group, _, w)) in ordered.iter().enumerate() {
        let Some(parent) = key(w).and_then(|k| parent_of(&k).map(|p| (k, p))) else {
            drawn.push((i, false, group));
            continue;
        };
        let (k, parent) = parent;
        if !emitted.insert(k.clone()) {
            continue;
        }
        let block = ordered[parent].0;
        drawn.push((parent, false, block));
        for m in &members[&k] {
            if *m != parent {
                drawn.push((*m, true, block));
            }
        }
    }
    for (n, (i, indented, block)) in drawn.iter().enumerate() {
        let first = n == 0 || drawn[n - 1].2 != *block;
        let next = drawn.get(n + 1);
        let last = next.is_none_or(|d| d.2 != *block);
        let (name, rail) = name_and_rail(parts, block);
        let head = match (indented, first) {
            (true, _) => Head::Child,
            (false, true) => Head::Corner(name),
            (false, false) => Head::Connector,
        };
        let w = ordered[*i].2;
        let mut tokens = card(rail, head, "", last && !indented, last && next.is_some());
        let home = homes.contains(w.workspace_id.as_str()) && w.label.ends_with(HOME_MARK);
        tokens.push(("hp_home".into(), home.then(|| "home".to_string())));
        out.spaces.push((w.workspace_id.clone(), tokens));
    }
    out
}

/// The `workspace.move` calls that turn `current` into `wanted`: each Space
/// in turn goes to its index, so at most one call per misplaced Space.
pub fn moves(current: &[String], wanted: &[String]) -> Vec<(String, usize)> {
    let mut now = current.to_vec();
    let mut out = Vec::new();
    for (i, id) in wanted.iter().enumerate() {
        let Some(at) = now.iter().position(|x| x == id) else {
            continue;
        };
        if at != i && i < now.len() {
            let moved = now.remove(at);
            now.insert(i, moved);
            out.push((id.clone(), i));
        }
    }
    out
}

/// Home Spaces whose label lacks [`HOME_MARK`] (made before 0.2.19, or
/// renamed by hand): `(id, label with the mark)`.
pub fn unmarked_homes(parts: &Parts, workspaces: &[Workspace]) -> Vec<(String, String)> {
    parts
        .values()
        .filter_map(|part| workspaces.iter().find(|w| !part.home.is_empty() && w.workspace_id == part.home))
        .filter(|w| !w.label.is_empty() && !w.label.ends_with(HOME_MARK))
        .map(|w| (w.workspace_id.clone(), format!("{}{HOME_MARK}", w.label)))
        .collect()
}

/// Tokens sent per `(kind, id)`, so an unchanged card is re-sent only to keep
/// its TTL alive.
#[derive(Default)]
pub struct Sent(HashMap<(String, String), (Tokens, Instant)>);

const RESEND: Duration = Duration::from_millis(crate::sidebar::TOKEN_TTL_MS / 3);

impl Sent {
    fn due(&mut self, key: (String, String), tokens: &Tokens) -> bool {
        match self.0.get(&key) {
            Some((last, at)) if last == tokens && at.elapsed() < RESEND => false,
            _ => {
                self.0.insert(key, (tokens.clone(), Instant::now()));
                true
            }
        }
    }
}

/// Herdr takes at most 16 tokens per report.
const PER_REPORT: usize = 16;

pub fn report(herdr: &Herdr, kind: &str, id: &str, tokens: &Tokens) {
    let ttl = crate::sidebar::TOKEN_TTL_MS.to_string();
    for chunk in tokens.chunks(PER_REPORT) {
        let mut args: Vec<String> = [kind, "report-metadata", id, "--source", crate::herdr::SOURCE, "--ttl-ms", &ttl].iter().map(|s| s.to_string()).collect();
        for (name, value) in chunk {
            match value {
                Some(value) => args.extend(["--token".to_string(), format!("{name}={value}")]),
                None => args.extend(["--clear-token".to_string(), name.clone()]),
            }
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let _ = herdr.call(&args, CALL_TIMEOUT);
    }
}

/// Clears every token this module and the 0.2.17/0.2.18 layout wrote.
pub fn clear(herdr: &Herdr, kind: &str, id: &str) {
    if id.is_empty() {
        return;
    }
    let all: Tokens = tokens().into_iter().chain(LEGACY.map(String::from)).map(|t| (t, None)).collect();
    report(herdr, kind, id, &all);
}

/// Lays out one session: marks home Spaces, moves Spaces into blocks and
/// writes every card's rail tokens. Nothing when no project lives in the
/// session.
pub fn apply(herdr: &Herdr, socket: &str, parts: &Parts, agents: &[Agent], sent: &mut Sent) {
    if parts.is_empty() {
        return;
    }
    let mut workspaces = herdr.workspace_list().unwrap_or_default();
    for (id, label) in unmarked_homes(parts, &workspaces) {
        if herdr.workspace_rename(&id, &label).is_ok()
            && let Some(w) = workspaces.iter_mut().find(|w| w.workspace_id == id)
        {
            w.label = label;
        }
    }
    let layout = plan(parts, agents, &workspaces);
    let current: Vec<String> = workspaces.iter().map(|w| w.workspace_id.clone()).collect();
    for (id, index) in moves(&current, &layout.order) {
        if herdr.workspace_move(&id, index).is_err() {
            break;
        }
    }
    for (pane, tokens) in &layout.agents {
        if sent.due((format!("{socket} pane"), pane.clone()), tokens) {
            report(herdr, "pane", pane, tokens);
        }
    }
    for (space, tokens) in &layout.spaces {
        if sent.due((format!("{socket} workspace"), space.clone()), tokens) {
            report(herdr, "workspace", space, tokens);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::WorkspaceWorktree;

    fn agent(pane: &str) -> Agent {
        Agent { pane_id: pane.into(), ..Agent::default() }
    }

    fn space(id: &str, repo: &str, linked: bool) -> Workspace {
        let worktree = (!repo.is_empty()).then(|| WorkspaceWorktree { repo_key: repo.into(), checkout_path: String::new(), is_linked_worktree: linked });
        Workspace { workspace_id: id.into(), label: id.into(), worktree, ..Workspace::default() }
    }

    /// The tokens a card sets, ignoring the ones it clears.
    fn set(tokens: &Tokens) -> BTreeMap<&str, &str> {
        tokens.iter().filter_map(|(n, v)| v.as_deref().map(|v| (n.as_str(), v))).collect()
    }

    fn row(pane: &str, key: String, sub: &str) -> PaneRow {
        PaneRow { pane: pane.into(), key, sub: sub.into() }
    }

    fn parts() -> Parts {
        let mut parts = Parts::new();
        parts.insert(
            "beta".into(),
            ProjectPart { name: "Beta".into(), rail: Rail::Idle, panes: vec![row("b1", coordinator_key("beta", "b1"), "")], home: "wb".into(), spaces: vec!["wb".into()] },
        );
        parts.insert(
            "alpha".into(),
            ProjectPart {
                name: "Alpha".into(),
                rail: Rail::Needs,
                panes: vec![
                    row("a2", thread_key("alpha", 4, "t-0002"), "~40% · Writing"),
                    row("a1", coordinator_key("alpha", "a1"), ""),
                    row("a3", thread_key("alpha", 1, "t-0003"), "review · report"),
                ],
                home: "wa".into(),
                spaces: vec!["wa".into(), "repo".into(), "wt2".into(), "wt3".into()],
            },
        );
        parts
    }

    #[test]
    fn agents_form_one_rail_per_project_and_others_last() {
        let agents: Vec<Agent> = ["x1", "a2", "b1", "a3", "x2", "a1"].into_iter().map(agent).collect();
        let layout = plan(&parts(), &agents, &[]);
        let order: Vec<&str> = layout.agents.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(order, ["a1", "a3", "a2", "b1", "x1", "x2"]);
        let t: HashMap<&str, BTreeMap<&str, &str>> = layout.agents.iter().map(|(p, t)| (p.as_str(), set(t))).collect();
        // Alpha needs you: a heavy red rail from corner to end.
        assert_eq!(t["a1"]["hp_top_n"], "\u{2800}\u{2800}┏━ Alpha");
        assert!(!t["a1"].contains_key("hp_con_n") && !t["a1"].contains_key("hp_end_n"));
        assert_eq!(t["a3"]["hp_con_n"], "\u{2800}\u{2800}┃");
        assert_eq!(t["a3"]["hp_sub_n"], "┃ review · report");
        assert_eq!(t["a2"]["hp_end_n"], "┗━");
        assert_eq!(t["a2"]["hp_gap"], BLANK, "a gap before the next block");
        // Beta is idle, alone: corner and end on one card.
        assert_eq!(t["b1"]["hp_top_i"], "\u{2800}\u{2800}┌─ Beta");
        assert_eq!(t["b1"]["hp_end_i"], "└─");
        assert!(!t["b1"].contains_key("hp_sub_i"), "no sub-line without text");
        // Everything else: a dashed `other` rail, and no gap after the last.
        assert_eq!(t["x1"]["hp_top_o"], "\u{2800}\u{2800}┌╌ other");
        assert_eq!(t["x2"]["hp_con_o"], "\u{2800}\u{2800}╎");
        assert_eq!(t["x2"]["hp_end_o"], "└╌");
        assert!(!t["x2"].contains_key("hp_gap"));
        // Every card clears the other states' tokens.
        let a1 = &layout.agents[0].1;
        assert!(a1.iter().any(|(n, v)| n == "hp_top_i" && v.is_none()));
        // The sort key the agent view uses reproduces this order.
        let keys: Vec<&str> = layout.agents.iter().map(|(_, t)| t.iter().find(|(n, _)| n == "hp_group").unwrap().1.as_deref().unwrap()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn spaces_share_the_rail_and_worktrees_join_it() {
        let mut workspaces = vec![
            space("mine", "", false),
            space("wt3", "r", true),
            space("wb", "", false),
            space("repo", "r", false),
            space("wa", "", false),
            space("wt2", "r", true),
            space("other2", "", false),
        ];
        workspaces[4].label = format!("Alpha{HOME_MARK}");
        let layout = plan(&parts(), &[], &workspaces);
        assert_eq!(layout.order, ["wa", "repo", "wt2", "wt3", "wb", "mine", "other2"]);
        let t: HashMap<&str, BTreeMap<&str, &str>> = layout.spaces.iter().map(|(p, t)| (p.as_str(), set(t))).collect();
        assert_eq!(t["wa"]["hp_top_n"], "\u{2800}\u{2800}┏━ Alpha");
        assert_eq!(t["wa"]["hp_home"], "home", "a marked home label says home");
        assert!(!t["wb"].contains_key("hp_home"), "an unmarked label keeps its own name");
        assert_eq!(t["repo"]["hp_con_n"], "\u{2800}\u{2800}┃");
        assert!(t["wt2"].is_empty(), "Herdr draws a worktree child on the rail");
        // The block ends in a worktree child: Herdr's `└─` closes it.
        assert_eq!(t["wt3"].get("hp_end_n"), None);
        assert_eq!(t["wt3"]["hp_gap"], BLANK);
        assert_eq!(t["wb"]["hp_top_i"], "\u{2800}\u{2800}┌─ Beta");
        assert_eq!(t["wb"]["hp_end_i"], "└─");
        assert_eq!(t["mine"]["hp_top_o"], "\u{2800}\u{2800}┌╌ other");
        assert_eq!(t["other2"]["hp_end_o"], "└╌");
        assert!(!t["other2"].contains_key("hp_gap"));
    }

    #[test]
    fn unmarked_home_labels_get_the_mark() {
        let mut marked = space("wb", "", false);
        marked.label = format!("Beta{HOME_MARK}");
        let mut plain = space("wa", "", false);
        plain.label = "Alpha".into();
        assert_eq!(unmarked_homes(&parts(), &[plain, marked, space("repo", "r", false)]), [("wa".to_string(), format!("Alpha{HOME_MARK}"))]);
    }

    #[test]
    fn a_card_never_passes_herdrs_token_limit_in_one_report() {
        assert!(tokens().len() > PER_REPORT, "the chunking in report() is needed");
        assert!(LEGACY.iter().all(|t| !tokens().iter().any(|n| n == t)));
    }

    #[test]
    fn moves_are_minimal_and_reach_the_wanted_order() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert!(moves(&s(&["a", "b", "c"]), &s(&["a", "b", "c"])).is_empty());
        let current = s(&["c", "a", "b", "d"]);
        let wanted = s(&["a", "b", "c", "d"]);
        let steps = moves(&current, &wanted);
        let mut now = current.clone();
        for (id, i) in &steps {
            // Herdr's insert index is a gap in the list before removal.
            let at = now.iter().position(|x| x == id).unwrap();
            let target = if at < *i { i - 1 } else { *i };
            let moved = now.remove(at);
            now.insert(target, moved);
        }
        assert_eq!(now, wanted);
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn no_project_means_no_layout() {
        assert_eq!(plan(&Parts::new(), &[agent("x")], &[space("w", "", false)]), Plan::default());
    }
}
