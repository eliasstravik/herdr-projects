//! Groups Herdr's sidebar by project with nothing but order and one head row
//! per project, the same in the agents and the Spaces list. The head is the
//! project's own row: its home Space and its coordinator show the project
//! name in bold, the rows after them are Herdr's own. Every row a card shows
//! is that item's own content, so focus and selection light only the item.
//!
//! The bold comes from a config rule keyed on an invisible mark in the name:
//! [`HOME_MARK`] ends every home Space label, [`HEAD_MARK`] every coordinator
//! display name. A mark in the name, not a list of names in the config,
//! because a client draws another machine's rows with its own config.
//!
//! Agents sort by `$hp_group` in the agent view; Spaces are moved into one
//! block per project with `workspace.move`. Anything in no project comes last.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::herdr::{Agent, CALL_TIMEOUT, Herdr, Workspace};

/// Ends every home Space label; the Space rows make a label that contains it
/// the project's bold head.
pub const HOME_MARK: char = '\u{2800}';
/// Ends every coordinator's display name; the agent rows make a name that
/// contains it the project's bold head. Zero-width, so it takes no cell.
pub const HEAD_MARK: char = '\u{200B}';
/// The group key of whatever belongs to no project: sorts after every slug.
const OTHER: &str = "~";

/// Every token this module writes.
pub fn tokens() -> Vec<String> {
    ["hp_sub", "hp_group"].map(String::from).to_vec()
}

/// Tokens of earlier layouts: the 0.2.17/0.2.18 headings and the 0.2.19
/// rails. Cleared with a pane's or Space's own tokens, and otherwise left to
/// expire (they had a TTL and have no rows any more).
pub fn legacy() -> Vec<String> {
    let mut all: Vec<String> = ["hp_top", "hp_other", "hp_note", "hp_tail", "hp", "hp_state", "hp_activity", "hp_gap", "hp_home"].map(String::from).to_vec();
    for slot in ["top", "con", "sub", "end"] {
        for rail in ["n", "w", "i", "o"] {
            all.push(format!("hp_{slot}_{rail}"));
        }
    }
    all
}

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
    /// The Space order Herdr should end up with.
    pub order: Vec<String>,
}

/// The whole layout of one session. Agents: projects by slug, each in its own
/// key order, then every other agent in Herdr's order. Spaces: the same
/// blocks, then every other Space in Herdr's order.
pub fn plan(parts: &Parts, agents: &[Agent], workspaces: &[Workspace]) -> Plan {
    let mut out = Plan::default();
    if parts.is_empty() {
        return out;
    }
    let mut owner: HashMap<&str, &PaneRow> = HashMap::new();
    for part in parts.values() {
        for row in &part.panes {
            owner.entry(row.pane.as_str()).or_insert(row);
        }
    }
    let mut rows: Vec<(String, &str, &str)> = agents
        .iter()
        .enumerate()
        .map(|(i, a)| match owner.get(a.pane_id.as_str()) {
            Some(row) => (row.key.clone(), a.pane_id.as_str(), row.sub.as_str()),
            None => (format!("{OTHER}!{i:04}"), a.pane_id.as_str(), ""),
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    for (key, pane, sub) in rows {
        let tokens = vec![("hp_sub".to_string(), (!sub.is_empty()).then(|| sub.to_string())), ("hp_group".to_string(), Some(key))];
        out.agents.push((pane.to_string(), tokens));
    }

    // Spaces: a stable sort into blocks.
    let mut space_owner: HashMap<&str, (&str, usize)> = HashMap::new();
    for (slug, part) in parts {
        for (i, id) in part.spaces.iter().enumerate() {
            space_owner.entry(id.as_str()).or_insert((slug.as_str(), i));
        }
    }
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
    let homes: HashSet<&str> = parts.values().map(|p| p.home.as_str()).filter(|h| !h.is_empty()).collect();
    workspaces
        .iter()
        .filter(|w| homes.contains(w.workspace_id.as_str()))
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

/// Clears every token this module and the earlier layouts wrote.
pub fn clear(herdr: &Herdr, kind: &str, id: &str) {
    if id.is_empty() {
        return;
    }
    let all: Tokens = tokens().into_iter().chain(legacy()).map(|t| (t, None)).collect();
    report(herdr, kind, id, &all);
}

/// Lays out one session: marks home Spaces, moves Spaces into blocks and
/// writes every agent's sort key and sub-line. Nothing when no project lives
/// in the session.
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

    fn row(pane: &str, key: String, sub: &str) -> PaneRow {
        PaneRow { pane: pane.into(), key, sub: sub.into() }
    }

    fn parts() -> Parts {
        let mut parts = Parts::new();
        parts.insert("beta".into(), ProjectPart { panes: vec![row("b1", coordinator_key("beta", "b1"), "")], home: "wb".into(), spaces: vec!["wb".into()] });
        parts.insert(
            "alpha".into(),
            ProjectPart {
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

    fn get<'a>(tokens: &'a Tokens, name: &str) -> Option<&'a str> {
        tokens.iter().find(|(n, _)| n == name).and_then(|(_, v)| v.as_deref())
    }

    #[test]
    fn agents_sort_into_projects_with_their_own_sub_lines_and_others_last() {
        let agents: Vec<Agent> = ["x1", "a2", "b1", "a3", "x2", "a1"].into_iter().map(agent).collect();
        let layout = plan(&parts(), &agents, &[]);
        let order: Vec<&str> = layout.agents.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(order, ["a1", "a3", "a2", "b1", "x1", "x2"]);
        let t: HashMap<&str, &Tokens> = layout.agents.iter().map(|(p, t)| (p.as_str(), t)).collect();
        assert_eq!(get(t["a3"], "hp_sub"), Some("review · report"));
        assert_eq!(get(t["a1"], "hp_sub"), None, "an empty sub-line is cleared, so no row is drawn");
        assert!(t["a1"].iter().any(|(n, v)| n == "hp_sub" && v.is_none()));
        // No card carries a row of ours besides its own sub-line.
        assert!(layout.agents.iter().all(|(_, t)| t.iter().all(|(n, _)| n == "hp_sub" || n == "hp_group")));
        // The sort key the agent view uses reproduces this order.
        let keys: Vec<&str> = layout.agents.iter().map(|(_, t)| get(t, "hp_group").unwrap()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn spaces_move_into_project_blocks_and_others_keep_their_order_last() {
        let workspaces = vec![
            space("mine", "", false),
            space("wt3", "r", true),
            space("wb", "", false),
            space("repo", "r", false),
            space("wa", "", false),
            space("wt2", "r", true),
            space("other2", "", false),
        ];
        let layout = plan(&parts(), &[], &workspaces);
        assert_eq!(layout.order, ["wa", "repo", "wt2", "wt3", "wb", "mine", "other2"]);
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
    fn clearing_covers_every_earlier_layout_and_fits_herdrs_token_limit_in_chunks() {
        let all = legacy();
        for old in ["hp_top", "hp_top_w", "hp_end_o", "hp_con_n", "hp_sub_i", "hp_gap", "hp_home", "hp_activity"] {
            assert!(all.iter().any(|t| t == old), "{old}");
        }
        assert!(tokens().len() + all.len() > PER_REPORT, "the chunking in report() is needed");
        assert!(all.iter().all(|t| !tokens().contains(t)));
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
