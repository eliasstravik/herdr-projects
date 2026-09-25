//! Groups Herdr's sidebar by project. Herdr 0.9.1 draws neither group
//! headings nor per-token groups, so the ticker builds them from what a
//! plugin can reach: the agent view sorts agents by `$hp_group`, spaces are
//! moved into one block per project, and the first card of each block carries
//! a heading row (`$hp_top`, `$hp_other`, `$hp_note`). Every other card gets
//! a blank `$hp_top`, so a card's own rows always start in the same column,
//! and the last card before the next block gets a blank `$hp_tail`. Anything
//! that belongs to no project forms the `other` block, last.

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use crate::herdr::{Agent, CALL_TIMEOUT, Herdr, Workspace};

/// Herdr drops a token whose value is only whitespace; U+2800 (braille blank)
/// is kept and draws as an empty cell.
pub const BLANK: &str = "\u{2800}";
pub const HEAD_MARK: &str = "▍";
/// The group key of whatever belongs to no project: sorts after every slug.
const OTHER: &str = "~";
/// Every token this module writes.
pub const TOKENS: [&str; 5] = ["hp_group", "hp_top", "hp_other", "hp_note", "hp_tail"];

/// One project's part of a session, as its tick saw it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectPart {
    pub name: String,
    /// The project line, `2 need you · 1 working`, or `paused`.
    pub note: String,
    /// The project's agents in panel order: `(pane id, order key)`.
    pub panes: Vec<(String, String)>,
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
pub type Tokens = Vec<(&'static str, Option<String>)>;

#[derive(Debug, Default, PartialEq)]
pub struct Plan {
    pub agents: Vec<(String, Tokens)>,
    pub spaces: Vec<(String, Tokens)>,
    /// The Space order Herdr should end up with.
    pub order: Vec<String>,
}

fn heading(parts: &Parts, group: &str, note: bool) -> Tokens {
    match parts.get(group) {
        Some(part) => vec![
            ("hp_top", Some(format!("{HEAD_MARK}{}", part.name))),
            ("hp_other", None),
            ("hp_note", (note && !part.note.is_empty()).then(|| part.note.clone())),
        ],
        None => vec![("hp_top", None), ("hp_other", Some(format!("{HEAD_MARK}other"))), ("hp_note", None)],
    }
}

fn spacer() -> Tokens {
    vec![("hp_top", Some(BLANK.into())), ("hp_other", None), ("hp_note", None)]
}

fn nothing() -> Tokens {
    vec![("hp_top", None), ("hp_other", None), ("hp_note", None)]
}

fn tail(last_of_block: bool) -> (&'static str, Option<String>) {
    ("hp_tail", last_of_block.then(|| BLANK.to_string()))
}

/// The whole layout of one session. Agents: projects by slug, each in its own
/// key order, then every other agent in Herdr's order. Spaces: the same
/// blocks; a worktree Space Herdr draws under its repository Space stays
/// there and never carries a heading.
pub fn plan(parts: &Parts, agents: &[Agent], workspaces: &[Workspace]) -> Plan {
    let mut out = Plan::default();
    if parts.is_empty() {
        return out;
    }

    // Agents.
    let mut owner: HashMap<&str, (&str, &str)> = HashMap::new();
    for (slug, part) in parts {
        for (pane, key) in &part.panes {
            owner.entry(pane.as_str()).or_insert((slug.as_str(), key.as_str()));
        }
    }
    let mut rows: Vec<(String, String, &str)> = agents
        .iter()
        .enumerate()
        .map(|(i, a)| match owner.get(a.pane_id.as_str()) {
            Some((slug, key)) => ((*slug).to_string(), (*key).to_string(), a.pane_id.as_str()),
            None => (OTHER.to_string(), format!("{OTHER}!{i:04}"), a.pane_id.as_str()),
        })
        .collect();
    rows.sort_by(|a, b| a.1.cmp(&b.1));
    for (i, (group, key, pane)) in rows.iter().enumerate() {
        let first = i == 0 || rows[i - 1].0 != *group;
        let last = rows.get(i + 1).is_some_and(|next| next.0 != *group);
        let mut tokens = if first { heading(parts, group, true) } else { spacer() };
        tokens.push(tail(last));
        tokens.push(("hp_group", Some(key.clone())));
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
    let mut emitted = std::collections::HashSet::new();
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
        let first_top = !indented && !drawn[..n].iter().rev().find(|d| !d.1).is_some_and(|d| d.2 == *block);
        let last = drawn.get(n + 1).is_some_and(|next| next.2 != *block);
        let mut tokens = match (indented, first_top) {
            (true, _) => nothing(),
            (false, true) => heading(parts, block, false),
            (false, false) => spacer(),
        };
        tokens.push(tail(last));
        out.spaces.push((ordered[*i].2.workspace_id.clone(), tokens));
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

fn report(herdr: &Herdr, kind: &str, id: &str, tokens: &Tokens) {
    let ttl = crate::sidebar::TOKEN_TTL_MS.to_string();
    let mut args: Vec<String> = [kind, "report-metadata", id, "--source", crate::herdr::SOURCE, "--ttl-ms", &ttl].iter().map(|s| s.to_string()).collect();
    for (name, value) in tokens {
        match value {
            Some(value) => args.extend(["--token".to_string(), format!("{name}={value}")]),
            None => args.extend(["--clear-token".to_string(), name.to_string()]),
        }
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let _ = herdr.call(&args, CALL_TIMEOUT);
}

/// Lays out one session: moves Spaces into blocks and writes every card's
/// grouping tokens. Nothing when no project lives in the session.
pub fn apply(herdr: &Herdr, socket: &str, parts: &Parts, agents: &[Agent], sent: &mut Sent) {
    if parts.is_empty() {
        return;
    }
    let workspaces = herdr.workspace_list().unwrap_or_default();
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
        Workspace { workspace_id: id.into(), worktree, ..Workspace::default() }
    }

    fn value<'a>(tokens: &'a Tokens, name: &str) -> Option<&'a str> {
        tokens.iter().find(|(n, _)| *n == name).and_then(|(_, v)| v.as_deref())
    }

    fn parts() -> Parts {
        let mut parts = Parts::new();
        parts.insert(
            "beta".into(),
            ProjectPart { name: "Beta".into(), note: "idle".into(), panes: vec![("b1".into(), coordinator_key("beta", "b1"))], spaces: vec!["wb".into()] },
        );
        parts.insert(
            "alpha".into(),
            ProjectPart {
                name: "Alpha".into(),
                note: "1 need you".into(),
                panes: vec![("a2".into(), thread_key("alpha", 4, "t-0002")), ("a1".into(), coordinator_key("alpha", "a1")), ("a3".into(), thread_key("alpha", 1, "t-0003"))],
                spaces: vec!["wa".into(), "repo".into(), "wt2".into(), "wt3".into()],
            },
        );
        parts
    }

    #[test]
    fn agents_form_one_block_per_project_with_a_heading_and_others_last() {
        let agents: Vec<Agent> = ["x1", "a2", "b1", "a3", "x2", "a1"].into_iter().map(agent).collect();
        let layout = plan(&parts(), &agents, &[]);
        let order: Vec<&str> = layout.agents.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(order, ["a1", "a3", "a2", "b1", "x1", "x2"]);
        let tokens: HashMap<&str, &Tokens> = layout.agents.iter().map(|(p, t)| (p.as_str(), t)).collect();
        assert_eq!(value(tokens["a1"], "hp_top"), Some("▍Alpha"));
        assert_eq!(value(tokens["a1"], "hp_note"), Some("1 need you"));
        assert_eq!(value(tokens["a3"], "hp_top"), Some(BLANK));
        assert_eq!(value(tokens["a3"], "hp_tail"), None);
        assert_eq!(value(tokens["a2"], "hp_tail"), Some(BLANK), "last of a block before another");
        assert_eq!(value(tokens["b1"], "hp_top"), Some("▍Beta"));
        assert_eq!(value(tokens["x1"], "hp_other"), Some("▍other"));
        assert_eq!(value(tokens["x1"], "hp_top"), None);
        assert_eq!(value(tokens["x2"], "hp_top"), Some(BLANK));
        assert_eq!(value(tokens["x2"], "hp_tail"), None, "the very last card has no tail");
        // The sort key the agent view uses reproduces this order.
        let mut keys: Vec<&str> = layout.agents.iter().map(|(_, t)| value(t, "hp_group").unwrap()).collect();
        let sorted = { let mut k = keys.clone(); k.sort(); k };
        assert_eq!(keys, sorted);
        keys.dedup();
        assert_eq!(keys.len(), 6);
    }

    #[test]
    fn spaces_move_into_blocks_and_worktrees_stay_under_their_repository() {
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
        let tokens: HashMap<&str, &Tokens> = layout.spaces.iter().map(|(p, t)| (p.as_str(), t)).collect();
        assert_eq!(value(tokens["wa"], "hp_top"), Some("▍Alpha"));
        assert_eq!(value(tokens["wa"], "hp_note"), None, "the home Space already shows the project line");
        assert_eq!(value(tokens["repo"], "hp_top"), Some(BLANK));
        assert_eq!(value(tokens["wt2"], "hp_top"), None, "worktree children never get a top row");
        assert_eq!(value(tokens["wt3"], "hp_tail"), Some(BLANK), "the last drawn entry of a block");
        assert_eq!(value(tokens["wb"], "hp_top"), Some("▍Beta"));
        assert_eq!(value(tokens["wb"], "hp_tail"), Some(BLANK));
        assert_eq!(value(tokens["mine"], "hp_other"), Some("▍other"));
        assert_eq!(value(tokens["other2"], "hp_tail"), None);
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
