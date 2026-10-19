//! herdr's machines sidebar, whose order the board lists machines in. herdr shows Local first, then the saved machines
//! enabled there in the order of its list (`herdr machine list --json`, kept on the computer the herdr window runs on;
//! herdr has no command that reorders it, but a plugin or an edit of that list does). Each machine sends its own list
//! with its tasks (`sent`; one on an older program, which sends none, has its herdr asked: `asked`); the board follows
//! the sidebar of the machine whose herdr window shows this one, else this machine's own, and lists the machines it
//! knows in that order, the ones that sidebar does not show after them (`order`), each named as that sidebar names it
//! (`names`). How to read it: docs/machine-order.md.
use super::{short_label, Machine};
use crate::json::*;

/// A saved machine as one machine's herdr lists it: `selected` is the one its herdr window shows now; `resolved` the
/// host its SSH connects to for that target (an alias in its SSH config as the host name it stands for); `addresses`
/// the addresses those hosts stand for (remote/lookup.rs), as the machine of the list looked them up.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry { pub id: String, pub label: String, pub target: String, pub session: String, pub enabled: bool, pub selected: bool, pub resolved: String, pub addresses: Vec<String> }
/// This machine as another machine's list may name it: its names and addresses (own_names.rs), and its herdr session.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Here { pub names: Vec<String>, pub session: String }

/// This machine's own herdr list, as another machine is sent it with the tasks: herdr's order, disabled ones too.
pub fn sent() -> Value { json!(crate::herdr::Herdr::new(None).machines().iter().filter(|m| m.is_object()).take(64).map(|m| entry(m, resolve(&m.t("target")))).collect::<Vec<_>>()) }
fn entry(m: &Value, resolved: String) -> Value {
    let addresses = super::lookup::addresses(&[crate::webui::pcs::address(&m.t("target"), 0).1, resolved.clone()]);
    json!({ "id": m.t("id"), "label": m.t("label"), "target": m.t("target"), "session": m.t("session"), "enabled": m.at("enabled") != &json!(false), "selected": m.at("selected") == &json!(true), "resolved": resolved, "addresses": addresses })
}
/// A list as another machine sent it, its entries that came without the addresses of their hosts (a program there that
/// sends none) given them as this machine looks them up: a Tailscale name stands for the same address on both.
pub fn looked_up(list: &mut Value) {
    for e in list.as_array_mut().into_iter().flatten().filter(|e| e.is_object() && !e.at("addresses").is_array()) {
        e["addresses"] = json!(super::lookup::addresses(&[crate::webui::pcs::address(&e.t("target"), 0).1, e.t("resolved")]));
    }
}
/// The herdr list of a machine whose answer came without one (an older program there, which sends none), asked of
/// its herdr itself over the same SSH: herdr's order and the machine its window shows, an SSH alias there unresolved.
/// None when its herdr gives no list (none found, or older than its machines).
pub fn asked(m: &Machine) -> Option<Value> {
    let (cmd, args) = super::ssh_command(m, r#"printf "\n"; herdr machine list --json | tr -d "\n"; printf "\n""#, false);
    let mut c = std::process::Command::new(cmd);
    c.args(args);
    let end = super::run_ended(c, 10000);
    if end.code != Some(0) { return None; }
    let j = end.stdout.lines().rev().find_map(|l| serde_json::from_str::<Value>(l.trim()).ok().filter(|v| v.is_array() || v.at("machines").is_array()))?;
    let list = j.as_array().cloned().unwrap_or_else(|| j.arr("machines").to_vec());
    Some(json!(list.iter().filter(|m| m.is_object()).take(64).map(|m| entry(m, String::new())).collect::<Vec<_>>()))
}
/// The host SSH connects to for a target, as `ssh -G` says (it reads the SSH config, connects nowhere); "" when it
/// cannot say, and with the tests' stand-in for ssh.
fn resolve(target: &str) -> String {
    if target.is_empty() || target.starts_with('-') || std::env::var_os("HERDR_PROJECTS_SSH").is_some() { return String::new(); }
    let mut c = std::process::Command::new("ssh");
    c.args(["-G", target]);
    let end = super::run_ended(c, 3000);
    if end.code != Some(0) { return String::new(); }
    end.stdout.lines().find_map(|l| l.strip_prefix("hostname ")).unwrap_or("").trim().to_lowercase()
}
/// A machine's list as it was sent; None from a machine that sends none (an older plugin).
pub fn read(v: &Value) -> Option<Vec<Entry>> {
    Some(v.as_array()?.iter().filter(|e| e.is_object()).map(|e| Entry { id: e.t("id"), label: e.t("label"), target: e.t("target"), session: e.t("session"), enabled: e.at("enabled") != &json!(false), selected: e.at("selected") == &json!(true), resolved: e.t("resolved"), addresses: e.arr("addresses").iter().filter_map(|a| a.as_str()).map(String::from).collect() }).collect())
}

/// A host as compared: lowercase, without a final dot or an address's brackets.
fn norm(h: &str) -> String { h.trim().trim_end_matches('.').trim_start_matches('[').trim_end_matches(']').to_lowercase() }
/// Two names of one host: the same whole name (an address only so); a bare name (no domain) is also the first part of
/// a whole one, as `mac-mini` is of `mac-mini.tail1234.ts.net`. `build.prod.example` is never `build.dev.example`.
/// This machine's names are only the ones it has (own_names.rs): a bare one is never cut from a whole one.
pub fn same_host(a: &str, b: &str) -> bool {
    let (a, b) = (norm(a), norm(b));
    if a.is_empty() || b.is_empty() { return false; }
    if a == b { return true; }
    let ip = |h: &str| h.parse::<std::net::IpAddr>().is_ok();
    let bare = |h: &str| !h.contains('.') && !h.contains(':');
    !ip(&a) && !ip(&b) && (bare(&a) || bare(&b)) && a.split('.').next() == b.split('.').next()
}
/// The hosts an entry connects to: as its target says, and as its machine's SSH resolves that.
fn hosts(e: &Entry) -> Vec<String> { [crate::webui::pcs::address(&e.target, 0).1, e.resolved.clone()].into_iter().filter(|h| !h.is_empty()).collect() }
fn session(s: &str) -> &str { if s == "default" { "" } else { s } }
/// Is this entry of another machine's list that machine of the board's? By herdr's id where both have the same one,
/// else by where it connects: the same host (`same_host`), port and herdr session.
fn same(e: &Entry, m: &Machine) -> bool {
    if !e.id.is_empty() && e.id == m.id { return true; }
    let (_, host, port) = crate::webui::pcs::address(&m.target, 0);
    hosts(e).iter().any(|h| same_host(h, &host)) && crate::webui::pcs::address(&e.target, 0).2 == port && session(&e.session) == session(&m.session)
}
/// Is this entry this machine, in its herdr session: by one of its names or addresses, else (`by_address`) by an
/// address its host stands for (remote/lookup.rs: a Mac named `Mac` that the list names by its Tailscale name).
fn is_here(e: &Entry, here: &Here, by_address: bool) -> bool {
    let hosts = if by_address { e.addresses.clone() } else { hosts(e) };
    session(&e.session) == session(&here.session) && hosts.iter().any(|h| here.names.iter().any(|n| same_host(h, n)))
}
/// The board's name for each entry: the machine it connects to (`same`), else one of the same name that no other entry
/// connects to; None for one the board does not know, and for `me` (this machine).
fn known(list: &[&Entry], ms: &[Machine], me: usize) -> Vec<Option<String>> {
    let by_place: Vec<Option<String>> = list.iter().enumerate().map(|(i, e)| if i == me { None } else { ms.iter().find(|m| same(e, m)).map(|m| m.label.clone()) }).collect();
    list.iter().enumerate().map(|(i, e)| if i == me { None } else { by_place[i].clone().or_else(|| ms.iter().find(|m| short_label(&e.label) == m.label && !by_place.contains(&Some(m.label.clone()))).map(|m| m.label.clone())) }).collect()
}
/// `from`'s sidebar as the board's machines: `from` itself first (its Local), then each entry herdr shows there that
/// the board knows, "" for this machine (the entry that is it: `is_here`, never a guess), each with the name that
/// sidebar shows it by. Also: whether `from`'s herdr window shows this machine now. None when this machine is not in
/// it, or not shown there.
fn as_board(from: &Machine, list: &[Entry], others: &[Machine], here: &Here) -> Option<(Vec<(String, String)>, bool)> {
    let list: Vec<&Entry> = list.iter().filter(|e| !same(e, from)).collect();
    // By name first; by address only an entry that is none of the board's other machines.
    let me = list.iter().position(|e| is_here(e, here, false)).or_else(|| list.iter().position(|e| is_here(e, here, true) && !others.iter().any(|m| same(e, m))))?;
    let names: Vec<Option<String>> = known(&list, others, me).into_iter().map(|n| n.filter(|l| *l != from.label)).collect();
    if !list[me].enabled { return None; }
    let mut order = vec![(from.label.clone(), LOCAL.to_string())];
    for (i, e) in list.iter().enumerate() {
        let name = if i == me { Some(String::new()) } else { names[i].clone() };
        if let Some(n) = name.filter(|n| e.enabled && !order.iter().any(|o| o.0 == *n)) { order.push((n, e.label.trim().to_string())); }
    }
    Some((order, list[me].selected))
}
/// What herdr's sidebar calls the computer its window runs on.
pub const LOCAL: &str = "Local";
/// The sidebar the board follows: whose it is (`from`), the board's machines in its order, and the name it shows each
/// one by ("" none: a machine it does not show).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Followed { pub from: Option<String>, pub order: Vec<String>, pub names: std::collections::BTreeMap<String, String> }
/// The board's machines ("" this machine, else each other one's name) in the order of the sidebar it follows, and whose
/// sidebar that is ("" this machine's; None: none could be read, and the order is the board's own of before: this
/// machine, then the others by name). It follows a machine whose herdr window shows this one now (the one it followed
/// before if it still does), else the one it followed before while that sidebar still has this machine, else this
/// machine's own (`own`: herdr's list here in its order; None when the machines are named by hand). The machines that
/// sidebar does not show come after the rest, this machine first, then in `far`'s order (the board's, by name).
pub fn order(own: Option<&[String]>, far: &[(Machine, Option<Vec<Entry>>)], here: &Here, before: Option<&str>) -> (Option<String>, Vec<String>) {
    let f = followed(own, far, here, before);
    (f.from, f.order)
}
/// The same, with the names another machine's sidebar shows: its machines as they were saved there, its own computer
/// `Local`. This machine's own sidebar names none: the board's names stand (the window may not be this machine's).
pub fn followed(own: Option<&[String]>, far: &[(Machine, Option<Vec<Entry>>)], here: &Here, before: Option<&str>) -> Followed {
    let others: Vec<Machine> = far.iter().map(|(m, _)| m.clone()).collect();
    type Read = (String, Vec<(String, String)>, bool);
    let read: Vec<Read> = far.iter().filter_map(|(m, l)| as_board(m, l.as_deref()?, &others, here).map(|(o, shows)| (m.label.clone(), o, shows))).collect();
    let shows: Vec<&Read> = read.iter().filter(|r| r.2).collect();
    let pick = shows.iter().find(|r| Some(r.0.as_str()) == before).or(shows.first()).copied().or_else(|| read.iter().find(|r| Some(r.0.as_str()) == before));
    let (from, mut out): (Option<String>, Vec<(String, String)>) = match (pick, own) {
        (Some(r), _) => (Some(r.0.clone()), r.1.clone()),
        (None, Some(own)) => (Some(String::new()), std::iter::once(String::new()).chain(own.iter().filter(|l| others.iter().any(|m| m.label == **l)).cloned()).map(|l| (l, String::new())).collect()),
        (None, None) => (None, vec![]),
    };
    for h in std::iter::once(String::new()).chain(others.iter().map(|m| m.label.clone())) { if !out.iter().any(|o| o.0 == h) { out.push((h, String::new())); } }
    let names = out.iter().filter(|o| !o.1.is_empty()).cloned().collect();
    Followed { from, order: out.into_iter().map(|o| o.0).collect(), names }
}
/// A machine as the board names it where the sidebar it follows shows it (`sidebar`: that sidebar's name for it, ""
/// none): by that name, with the board's own beside it when the two differ (`Local (elias-macbook-pro-m1)`,
/// `Mac mini (Mac)`), else by the board's.
pub fn shown_as(board: &str, sidebar: &str) -> String {
    let same = |a: &str| a.eq_ignore_ascii_case(board) || super::short_label(a).eq_ignore_ascii_case(board);
    if sidebar.trim().is_empty() || same(sidebar) { board.to_string() } else if board.is_empty() { sidebar.to_string() } else { format!("{sidebar} ({board})") }
}
