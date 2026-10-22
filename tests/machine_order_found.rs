//! The machine order of HP-61: a Mac mini that calls itself `Mac`, saved in the M1's herdr by its Tailscale name, is
//! still found in the M1's sidebar by the address that name stands for, so the board follows the sidebar of the herdr
//! window that shows it, not its own list; with two herdrs that list the machines in different orders, the one whose
//! window shows this machine is followed, and each machine is named as that sidebar names it.
use crate::common;
use crate::far;
use common::board::open;
use common::*;
use far::{theirs, Far};
use serde_json::{json, Value};
use sw::board::update::{update, Event};
use sw::json::*;
use sw::remote::sidebar::{followed, order, shown_as, Entry, Here};
use sw::remote::Machine;

fn names(v: &[&str]) -> Vec<String> { v.iter().map(|s| s.to_string()).collect() }
fn m(label: &str) -> Machine { Machine { id: String::new(), target: format!("exedev@{label}"), session: "default".into(), label: label.into() } }
fn e(label: &str, target: &str) -> Entry { Entry { label: label.into(), target: target.into(), session: "default".into(), enabled: true, ..Default::default() } }
/// The Mac mini as a herdr elsewhere saves it: by its Tailscale name, which stands for its Tailscale address there.
fn mini(shown: bool, addresses: &[&str]) -> Entry { Entry { selected: shown, addresses: names(addresses), ..e("Mac mini", "elias@eliass-mac-mini") } }
fn sidebar(mini_at: usize, shown: bool, addresses: &[&str], rest: &[&str]) -> Vec<Entry> {
    let mut v: Vec<Entry> = rest.iter().map(|l| e(l, &format!("exedev@{l}"))).collect();
    v.insert(mini_at, mini(shown, addresses));
    v
}
/// The Mac mini: its host name is `Mac`, its addresses its LAN's and its Tailscale's.
fn here() -> Here { Here { names: names(&["mac", "192.168.1.20", "100.122.138.92"]), session: String::new() } }
const TS: [&str; 1] = ["100.122.138.92"];

#[test]
fn of_two_herdrs_in_different_orders_the_one_whose_window_shows_this_machine_is_followed_and_names_it() {
    // the Mac mini's own herdr list, in the order its machines were added
    let own = names(&["elias-macbook-pro-m1", "eliass-macbook-pro-3", "vm-str", "vm-not", "vm-kil", "vm-demo", "vm-ssa"]);
    let rest = ["vm-str", "vm-ssa", "vm-not", "vm-kil", "vm-demo"];
    let m1 = |shown: bool, a: &[&str]| sidebar(0, shown, a, &rest);
    let pro3 = |shown: bool| sidebar(2, shown, &TS, &["vm-demo", "vm-kil", "vm-not", "vm-ssa", "vm-str"]);
    let far = |m1: Vec<Entry>, pro3: Vec<Entry>| -> Vec<(Machine, Option<Vec<Entry>>)> {
        own.iter().map(|l| (m(l), Some(match l.as_str() { "elias-macbook-pro-m1" => m1.clone(), "eliass-macbook-pro-3" => pro3.clone(), _ => vec![] }))).collect()
    };
    // what the human saw: the M1 names the Mac mini by a name that is none of its own, no address with it, so the
    // board took its own list
    let (from, o) = order(Some(&own), &far(m1(true, &[]), pro3(false)), &here(), None);
    assert_eq!((from.as_deref(), o[..3].to_vec()), (Some(""), names(&["", "elias-macbook-pro-m1", "eliass-macbook-pro-3"])));
    // with the address that name stands for, the M1's window is found to show this machine, and its sidebar is followed
    let f = followed(Some(&own), &far(m1(true, &TS), pro3(false)), &here(), None);
    assert_eq!(f.from.as_deref(), Some("elias-macbook-pro-m1"));
    assert_eq!(f.order, names(&["elias-macbook-pro-m1", "", "vm-str", "vm-ssa", "vm-not", "vm-kil", "vm-demo", "eliass-macbook-pro-3"]), "the M1 (Local) first, the Mac mini where it stands there, the one it does not show last");
    assert_eq!((f.names["elias-macbook-pro-m1"].as_str(), f.names[""].as_str(), f.names.get("eliass-macbook-pro-3")), ("Local", "Mac mini", None));
    // the other laptop's window shows this machine now, the M1's another one: the other laptop's sidebar is followed
    let f = followed(Some(&own), &far(m1(false, &TS), pro3(true)), &here(), Some("elias-macbook-pro-m1"));
    assert_eq!(f.from.as_deref(), Some("eliass-macbook-pro-3"));
    assert_eq!(f.order, names(&["eliass-macbook-pro-3", "vm-demo", "vm-kil", "", "vm-not", "vm-ssa", "vm-str", "elias-macbook-pro-m1"]));
    // an address another of the board's machines connects to is never taken for this one (a LAN address in two places)
    let mut lan = m1(false, &[]);
    lan[1].addresses = names(&["192.168.1.20"]); lan[1].selected = true; // vm-str
    assert_eq!(order(Some(&own), &far(lan, pro3(false)), &here(), None).0.as_deref(), Some(""));
    // the names: the sidebar's, with the board's beside it where that is another
    assert_eq!((shown_as("elias-macbook-pro-m1", "Local"), shown_as("Mac", "Mac mini"), shown_as("vm-str", "vm-str"), shown_as("vm-str", "vm-str.tail1.ts.net"), shown_as("vm-ssa", "")),
        ("Local (elias-macbook-pro-m1)".to_string(), "Mac mini (Mac)".to_string(), "vm-str".to_string(), "vm-str".to_string(), "vm-ssa".to_string()));
}

fn herdr_list(w: &World, list: Value) { std::fs::create_dir_all(&w.fake.dir).unwrap(); std::fs::write(w.fake.dir.join("machines.json"), list.to_string()).unwrap(); }
fn saved(label: &str) -> Value { json!({ "id": "", "label": label, "target": format!("exedev@{label}"), "session": "default", "enabled": true, "selected": false }) }
fn headings(b: &common::board::Board) -> Vec<String> {
    b.s.rows.iter().filter_map(|r| match r { sw::board::state::Row::Head { name, machine: true, .. } => Some(name.clone()), _ => None }).collect()
}

#[test]
fn the_board_finds_itself_in_the_m1s_sidebar_by_the_address_its_tailscale_name_stands_for_and_lists_every_machine_so() {
    let w = world();
    let far = Far::new(&w.base);
    w.add("Mine");
    // An address of this machine, which the M1's resolver gives for the name it saved this machine by.
    let Some(mine) = sw::remote::own_names::own_names().into_iter().find(|n| n.parse::<std::net::IpAddr>().is_ok()) else { return };
    far.host("eliass-mac-mini", &[&mine]);
    const MINI: [&str; 6] = ["elias-macbook-pro-m1", "vm-str", "vm-not", "vm-kil", "vm-demo", "vm-ssa"];
    herdr_list(&w, json!(MINI.iter().map(|l| saved(l)).collect::<Vec<_>>()));
    for l in &MINI[1..] { far.tasks(l, "1.2.4", json!([theirs(2)])); }
    // the M1 sends its sidebar as 1.2.4 does, without addresses: they are looked up on this side
    let m1 = |rest: &[&str]| json!(std::iter::once(json!({ "id": "0123", "label": "Mac mini", "target": "elias@eliass-mac-mini", "session": "default", "enabled": true, "selected": true, "resolved": "eliass-mac-mini" })).chain(rest.iter().map(|l| saved(l))).collect::<Vec<_>>());
    far.answers("elias-macbook-pro-m1", "list", &json!({ "host": "m1", "version": "1.2.4", "tasks": [theirs(1)], "sidebar": m1(&["vm-str", "vm-ssa", "vm-not", "vm-kil", "vm-demo"]) }));
    let mut b = open(&w.root, 60);
    b.cols = 200;
    update(&mut b.s, Event::Machines(sw::remote::machines()));
    for m in sw::remote::machines() { let label = m.label.clone(); let a = sw::remote::ask_list(&m, 20000); update(&mut b.s, Event::Remote { label, answer: a }); }
    // Alt+M: every machine comes first in its round
    while b.s.scope != "*" { b.press(&["alt+m"]); }
    assert_eq!(b.s.sidebar_from.as_deref(), Some("elias-macbook-pro-m1"));
    let here = sw::board::update::heads::label(&b.s, "");
    assert_eq!(headings(&b), vec!["Local (elias-macbook-pro-m1)".to_string(), format!("Mac mini ({here})"), "vm-str".into(), "vm-ssa".into(), "vm-not".into(), "vm-kil".into(), "vm-demo".into()]);
    let said = "Local (elias-macbook-pro-m1), vm-str, vm-ssa, vm-not, vm-kil, vm-demo";
    assert!(b.text().contains("\n All machines │ "), "the filter row names whose board: {}", b.text());
    assert_eq!(sw::webui::model::list::list_of(&b.s, &b.theme).at("head").s("machines"), said);
    // the scope key's round, which the web UI's Boards section follows too
    let mut round = vec![];
    for _ in 0..7 { b.press(&["alt+m"]); round.push(b.s.scope.clone()); }
    assert_eq!(round, names(&["elias-macbook-pro-m1", "", "vm-str", "vm-ssa", "vm-not", "vm-kil", "vm-demo"]));
    // what this machine sends another one: the addresses its herdr's machines stand for, as it looks them up
    far.host("vm-str", &["100.89.147.73"]);
    let sent = sw::remote::remote_list();
    assert_eq!(sent.arr("sidebar").iter().map(|x| (x.t("label"), x.arr("addresses").to_vec())).find(|x| x.0 == "vm-str"), Some(("vm-str".to_string(), vec![json!("100.89.147.73")])));
}
