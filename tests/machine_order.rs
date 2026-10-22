//! The machines in the order of herdr's sidebar: which machine's sidebar the board follows (the one whose herdr window
//! shows this machine, the one it followed before, this machine's own), which entry is this machine (never a guess)
//! and which machine each other one is (whole host names), where this machine stands in it, the ones it
//! does not show after the rest, a reorder followed at the next answer, the board's own order when none can be read,
//! and that order in the headings, the header, the scope key's round, the automations and the web board; a machine on
//! an older program, which sends no list, followed through its herdr (the Mac mini of HSW-228).
use crate::common;
use crate::far;
use common::board::open;
use common::*;
use far::{theirs, Far};
use serde_json::{json, Value};
use sw::board::update::{update, Event};
use sw::json::*;
use sw::remote::sidebar::{order, read, same_host, Entry, Here};
use sw::remote::Machine;

fn m(label: &str) -> Machine { Machine { id: String::new(), target: format!("me@{label}"), session: "default".into(), label: label.into() } }
fn e(label: &str, target: &str) -> Entry { Entry { label: label.into(), target: target.into(), session: "default".into(), enabled: true, ..Default::default() } }
fn picked(mut x: Entry) -> Entry { x.selected = true; x }
fn off(mut x: Entry) -> Entry { x.enabled = false; x }
fn names(v: &[&str]) -> Vec<String> { v.iter().map(|s| s.to_string()).collect() }
/// This machine as named in the tests: the Mac mini.
fn here() -> Here { Here { names: names(&["eliass-mac-mini.local", "eliass-mac-mini", "100.64.0.7"]), session: String::new() } }
fn named(n: &[&str]) -> Here { Here { names: names(n), session: String::new() } }
/// The board's machines on the Mac mini of the task: its own herdr list, in the order they were added.
fn mini() -> Vec<String> { names(&["m1", "vm-str", "vm-not", "vm-kil", "vm-demo", "vm-ssa"]) }
/// The MacBook's sidebar: the Mac mini (this machine), then the virtual machines as the user put them.
fn macbook(shows_mini: bool) -> Vec<Entry> {
    let mini = e("Mac mini", "elias@eliass-mac-mini");
    vec![if shows_mini { picked(mini) } else { mini }, e("vm-str", "exedev@vm-str"), e("vm-ssa", "exedev@vm-ssa"), e("vm-not", "exedev@vm-not"), e("vm-kil", "exedev@vm-kil"), e("vm-demo", "exedev@vm-demo")]
}
/// The other machines as the board keeps them (by name), m1 with the sidebar it sent.
fn far_of(m1: Option<Vec<Entry>>) -> Vec<(Machine, Option<Vec<Entry>>)> {
    let mut v: Vec<(Machine, Option<Vec<Entry>>)> = mini().iter().map(|l| (m(l), if l == "m1" { m1.clone() } else { Some(vec![]) })).collect();
    v.sort_by(|a, b| a.0.label.cmp(&b.0.label));
    v
}

#[test]
fn the_board_follows_the_sidebar_of_the_herdr_window_that_shows_this_machine_with_this_machine_where_it_stands_there() {
    let own = mini();
    let (from, o) = order(Some(&own), &far_of(Some(macbook(true))), &here(), None);
    assert_eq!(from.as_deref(), Some("m1"));
    assert_eq!(o, names(&["m1", "", "vm-str", "vm-ssa", "vm-not", "vm-kil", "vm-demo"]), "Local (the MacBook) first, then the Mac mini, then the rest as herdr shows them");
    // the user reorders the MacBook's sidebar: the next answer is followed
    let mut moved = macbook(true); moved.swap(1, 5);
    assert_eq!(order(Some(&own), &far_of(Some(moved)), &here(), None).1, names(&["m1", "", "vm-demo", "vm-ssa", "vm-not", "vm-kil", "vm-str"]));
    // this machine found by its whole name, a bare one, an address, or what an SSH alias there stands for; also among
    // machines the board does not know
    for (target, resolved) in [("me@eliass-mac-mini.tail1.ts.net", ""), ("me@100.64.0.7", ""), ("ssh://me@[100.64.0.7]:2222", ""), ("studio", "eliass-mac-mini.tail1.ts.net")] {
        let mut more = macbook(false); more[0] = e("work-laptop", "me@work-laptop");
        let mut studio = picked(e("studio", target)); studio.resolved = resolved.into(); more.push(studio);
        let (from, o) = order(Some(&own), &far_of(Some(more)), &here(), None);
        assert_eq!(from.as_deref(), Some("m1"), "{target}");
        assert_eq!(o, names(&["m1", "vm-str", "vm-ssa", "vm-not", "vm-kil", "vm-demo", ""]), "{target}");
    }
    // another herdr session on this machine's host is not this board
    let mut other = macbook(true); other[0].session = "agents".into();
    assert_eq!(order(Some(&own), &far_of(Some(other)), &here(), None).0.as_deref(), Some(""));
}

#[test]
fn without_a_window_that_shows_this_machine_the_one_followed_before_holds_else_this_machines_own_list_else_the_old_order() {
    let own = mini();
    // the MacBook shows another machine now: the board keeps the sidebar it followed
    let (from, o) = order(Some(&own), &far_of(Some(macbook(false))), &here(), Some("m1"));
    assert_eq!((from.as_deref(), o[1].as_str()), (Some("m1"), ""));
    // never followed: this machine's own herdr list, this machine (its Local) first
    let (from, o) = order(Some(&own), &far_of(Some(macbook(false))), &here(), None);
    assert_eq!(from.as_deref(), Some(""));
    assert_eq!(o, names(&["", "m1", "vm-str", "vm-not", "vm-kil", "vm-demo", "vm-ssa"]));
    // an older plugin sends no sidebar, and machines named by hand are no herdr list: this machine, then by name
    let (from, o) = order(None, &far_of(None), &here(), Some("m1"));
    assert_eq!(from, None);
    assert_eq!(o, names(&["", "m1", "vm-demo", "vm-kil", "vm-not", "vm-ssa", "vm-str"]));
    // a sidebar where this machine is turned off does not show it: it is not followed
    let mut hidden = macbook(true); hidden[0] = off(hidden[0].clone());
    assert_eq!(order(Some(&own), &far_of(Some(hidden)), &here(), None).0.as_deref(), Some(""));
}

#[test]
fn the_machines_a_sidebar_does_not_show_come_after_the_rest_this_machine_first_then_by_name() {
    let mut list = macbook(true);
    list[2] = off(list[2].clone()); // vm-ssa turned off on the MacBook
    list.remove(4);                 // vm-kil not saved there at all
    let (_, o) = order(Some(&mini()), &far_of(Some(list)), &here(), None);
    assert_eq!(o, names(&["m1", "", "vm-str", "vm-not", "vm-demo", "vm-kil", "vm-ssa"]));
    // a sidebar without this machine is not followed, whatever its window shows: an unknown machine is never taken for it
    let own = names(&["a", "b"]);
    let far = vec![(m("a"), Some(vec![e("b", "me@b"), picked(e("unrelated", "me@unrelated"))])), (m("b"), None)];
    assert_eq!(order(Some(&own), &far, &named(&["actual-board"]), None), (Some(String::new()), names(&["", "a", "b"])));
    // hosts that share their first part are told apart by the rest
    let far = vec![(m("a"), Some(vec![picked(e("Board", "me@actual-board")), e("prod", "me@build.prod.example"), e("dev", "me@build.dev.example")])),
                   (Machine { target: "me@build.dev.example".into(), ..m("build-dev") }, None), (Machine { target: "me@build.prod.example".into(), ..m("build-prod") }, None)];
    assert_eq!(order(None, &far, &named(&["actual-board"]), None).1, names(&["a", "", "build-prod", "build-dev"]));
    assert!(same_host("mac-mini", "Mac-Mini.tail1.ts.net.") && same_host("[fd7a::1]", "fd7a::1"));
    // this machine named build.prod.example is not the build.dev.example another window shows
    let here_prod = Here { names: sw::remote::own_names::host_names("build.prod.example"), session: String::new() };
    assert_eq!(here_prod.names, names(&["build.prod.example"]));
    assert_eq!(sw::remote::own_names::host_names("Eliass-Mac-mini.local."), names(&["eliass-mac-mini.local", "eliass-mac-mini"]));
    let far = vec![(m("laptop"), Some(vec![picked(e("dev", "me@build.dev.example"))]))];
    assert_eq!(order(Some(&names(&["laptop"])), &far, &here_prod, None), (Some(String::new()), names(&["", "laptop"])));
    let far = vec![(m("laptop"), Some(vec![picked(e("prod", "me@build.prod.example"))]))];
    assert_eq!(order(Some(&names(&["laptop"])), &far, &here_prod, None).0.as_deref(), Some("laptop"));
    assert!(!same_host("build.prod.example", "build.dev.example") && !same_host("10.0.0.1", "10.0.0.12") && !same_host("10", "10.0.0.1") && !same_host("", ""));
    // as sent: an older machine sends nothing; the fields herdr leaves out are read as herdr means them
    assert_eq!(read(&Value::Null), None);
    assert_eq!(read(&json!([{ "label": "a", "target": "me@a" }])).unwrap()[0], Entry { label: "a".into(), target: "me@a".into(), enabled: true, ..Default::default() });
}

fn herdr_list(w: &World, list: Value) { std::fs::create_dir_all(&w.fake.dir).unwrap(); std::fs::write(w.fake.dir.join("machines.json"), list.to_string()).unwrap(); }
fn saved(label: &str, selected: bool) -> Value { json!({ "id": "", "label": label, "target": format!("me@{label}"), "session": "default", "enabled": true, "selected": selected }) }
fn answer(b: &mut common::board::Board, label: &str, n: i64, sidebar: Value) { update(&mut b.s, Event::Remote { label: label.into(), answer: Ok(json!({ "host": label, "version": "0.2.0", "tasks": [theirs(n)], "sidebar": sidebar })) }); }
/// Alt+M round to every machine, which comes first in its round.
fn every(b: &mut common::board::Board) { for _ in 0..8 { if b.s.scope == "*" { return; } b.press(&["alt+m"]); } panic!("alt+m never showed every machine"); }
fn headings(b: &mut common::board::Board) -> Vec<String> {
    b.s.rows.iter().filter_map(|r| match r { sw::board::state::Row::Head { name, machine: true, .. } => Some(name.clone()), _ => None }).collect()
}

#[test]
fn every_list_of_machines_on_the_board_is_in_the_sidebars_order_and_follows_it_when_it_changes() {
    let w = world();
    let _far = Far::new(&w.base);
    w.add("Mine");
    herdr_list(&w, json!([saved("vm-c", false), saved("vm-a", false), saved("vm-b", false)]));
    let mut b = open(&w.root, 40);
    update(&mut b.s, Event::Machines(sw::remote::machines()));
    for (n, l) in ["vm-a", "vm-b", "vm-c"].iter().enumerate() { answer(&mut b, l, n as i64 + 1, json!([])); }
    every(&mut b);
    let here = sw::board::update::heads::label(&b.s, "");
    // this machine's own herdr list: its Local first, then the others as herdr lists them, not by name
    assert_eq!(headings(&mut b), vec![here.clone(), "vm-c".into(), "vm-a".into(), "vm-b".into()]);
    assert!(b.text().contains("\n All machines │ "), "the filter row names whose board: {}", b.text());
    let model = sw::webui::model::list::list_of(&b.s, &b.theme);
    assert_eq!(model.at("head").s("machines"), "vm-c, vm-a, vm-b");
    // the scope key goes round in that order too: every machine, then each one, this machine where herdr has it
    let mut round = vec![];
    for _ in 0..4 { b.press(&["alt+m"]); round.push(b.s.scope.clone()); }
    assert_eq!(round, names(&["", "vm-c", "vm-a", "vm-b"]));
    b.press(&["alt+m"]);
    assert_eq!(b.s.scope, "*");
    // vm-b's herdr window shows this machine: its sidebar is followed, vm-b itself (its Local) first
    let mine = json!({ "label": "this one", "target": format!("me@{}", sw::remote::check::hostname()), "session": "default", "enabled": true, "selected": true });
    answer(&mut b, "vm-b", 2, json!([saved("vm-a", false), mine.clone(), saved("vm-c", false)]));
    assert_eq!(b.s.sidebar_from.as_deref(), Some("vm-b"));
    // each named as that sidebar names it: vm-b its Local, this machine "this one"
    assert_eq!(headings(&mut b), vec!["Local (vm-b)".to_string(), "vm-a".into(), format!("this one ({here})"), "vm-c".into()]);
    // reordered there: followed at its next answer, and still when that window shows another machine meanwhile
    let mut later = mine.clone(); later["selected"] = json!(false);
    answer(&mut b, "vm-b", 2, json!([later, saved("vm-c", true), saved("vm-a", false)]));
    assert_eq!(headings(&mut b), vec!["Local (vm-b)".to_string(), format!("this one ({here})"), "vm-c".into(), "vm-a".into()]);
    // whose sidebar it was is kept with the view
    sw::board::run::prefs::save(&b.s);
    assert_eq!(sw::board::run::prefs::read(false).s("sidebarFrom"), "vm-b");
}

#[test]
fn machines_named_by_hand_keep_the_old_order_and_this_machine_sends_its_own_herdr_list() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-c, vm-a, vm-b" }), ..Default::default() });
    let _far = Far::new(&w.base);
    w.add("Mine");
    let mut b = open(&w.root, 40);
    update(&mut b.s, Event::Machines(sw::remote::machines()));
    for (n, l) in ["vm-a", "vm-b", "vm-c"].iter().enumerate() { answer(&mut b, l, n as i64 + 1, Value::Null); }
    every(&mut b);
    assert_eq!(b.s.sidebar_from, None);
    assert_eq!(headings(&mut b)[1..].to_vec(), names(&["vm-a", "vm-b", "vm-c"]));
    // what this machine sends another one: herdr's list here, in its order, with the machine its window shows
    herdr_list(&w, json!([saved("vm-c", false), { "id": "x", "label": "vm-a", "target": "me@vm-a", "session": "default", "enabled": false, "selected": true }]));
    let sent = sw::remote::remote_list();
    let got: Vec<(String, bool, bool)> = sent.arr("sidebar").iter().map(|x| (x.t("label"), x.b("enabled"), x.b("selected"))).collect();
    assert_eq!(got, vec![("vm-c".to_string(), true, false), ("vm-a".to_string(), false, true)]);
}

#[test]
fn the_sidebar_followed_before_holds_while_its_machine_has_not_answered_yet() {
    let w = world();
    let _far = Far::new(&w.base);
    herdr_list(&w, json!([saved("vm-c", false), saved("vm-b", false)]));
    let mut b = open(&w.root, 40);
    b.s.sidebar_from = Some("vm-b".into());
    update(&mut b.s, Event::Machines(sw::remote::machines()));
    assert_eq!(b.s.order[0], "", "nothing from vm-b yet: this machine's own list");
    assert_eq!(b.s.sidebar_from.as_deref(), Some("vm-b"));
    let mine = json!({ "label": "this one", "target": format!("me@{}", sw::remote::check::hostname()), "session": "default", "enabled": true, "selected": false });
    answer(&mut b, "vm-c", 1, json!([]));
    answer(&mut b, "vm-b", 2, json!([mine, saved("vm-c", false)]));
    assert_eq!(b.s.order, names(&["vm-b", "", "vm-c"]), "its window shows another machine now, and it is still followed");
}

/// The machines of the task, as the Mac mini's herdr lists them (in the order they were added) and as the MacBook's
/// sidebar shows them (its Local, the Mac mini, then the rest as the user put them).
const MINI: [&str; 6] = ["elias-macbook-pro-m1", "vm-str", "vm-not", "vm-kil", "vm-demo", "vm-ssa"];
fn sidebar_of_the_m1(order: &[&str]) -> Value {
    let mini = json!({ "id": "0123456789abcdef0123456789abcdef", "label": "Mac mini", "target": format!("elias@{}", sw::remote::check::hostname()), "session": "default", "enabled": true, "selected": true });
    json!(std::iter::once(mini).chain(order.iter().map(|l| saved(l, false))).collect::<Vec<_>>())
}
/// The M1 answers as a machine on an older program does: its tasks, no herdr list with them; its herdr gives its list.
fn m1_answers(far: &Far, order: &[&str]) {
    far.tasks("elias-macbook-pro-m1", "0.3.0", json!([theirs(1)]));
    // herdr's list, as it comes back over SSH: whatever a start-up file printed, then the list on one line
    far.ends("elias-macbook-pro-m1", "herdr-machines", &format!("Welcome to the M1\n{}\n", sidebar_of_the_m1(order)), "", 0);
}
fn ask_all(b: &mut common::board::Board) {
    for m in sw::remote::machines() { let label = m.label.clone(); let a = sw::remote::ask_list(&m, 20000); update(&mut b.s, Event::Remote { label, answer: a }); }
}

#[test]
fn a_machine_on_an_older_program_still_has_its_sidebar_followed_through_its_herdr_as_on_the_mac_mini_of_the_task() {
    let w = world();
    let far = Far::new(&w.base);
    w.add("Mine");
    herdr_list(&w, json!(MINI.iter().map(|l| saved(l, false)).collect::<Vec<_>>()));
    for l in &MINI[1..] { far.tasks(l, "0.3.0", json!([theirs(2)])); }
    m1_answers(&far, &["vm-str", "vm-ssa", "vm-not", "vm-kil", "vm-demo"]);
    let mut b = open(&w.root, 60);
    b.cols = 200;
    update(&mut b.s, Event::Machines(sw::remote::machines()));
    ask_all(&mut b);
    every(&mut b);
    assert_eq!(b.s.sidebar_from.as_deref(), Some("elias-macbook-pro-m1"));
    let here = sw::board::update::heads::label(&b.s, "");
    // what the user saw was the Mac mini's own list: elias-macbook-pro-m1, vm-str, vm-not, vm-kil, vm-demo, vm-ssa;
    // each is named as the M1's sidebar names it, the board's name beside it where that is another
    let local = "Local (elias-macbook-pro-m1)".to_string();
    assert_eq!(headings(&mut b), vec![local.clone(), format!("Mac mini ({here})"), "vm-str".into(), "vm-ssa".into(), "vm-not".into(), "vm-kil".into(), "vm-demo".into()]);
    assert_eq!(sw::webui::model::list::list_of(&b.s, &b.theme).at("head").s("machines"), "Local (elias-macbook-pro-m1), vm-str, vm-ssa, vm-not, vm-kil, vm-demo");
    // the user moves vm-demo up in the MacBook's herdr: followed at the M1's next answer
    m1_answers(&far, &["vm-demo", "vm-str", "vm-ssa", "vm-not", "vm-kil"]);
    ask_all(&mut b);
    assert_eq!(headings(&mut b)[2..].to_vec(), names(&["vm-demo", "vm-str", "vm-ssa", "vm-not", "vm-kil"]));
    // a machine whose program sends its list is not asked again; one whose herdr has no list keeps the order of before
    let mut sent = sidebar_of_the_m1(&["vm-kil", "vm-str", "vm-ssa", "vm-not", "vm-demo"]);
    sent[0]["resolved"] = json!("");
    far.answers("elias-macbook-pro-m1", "list", &json!({ "host": "m1", "version": "0.3.0", "tasks": [theirs(1)], "sidebar": sent }));
    let _ = std::fs::remove_file(far.dir.join("asked-herdr"));
    ask_all(&mut b);
    assert_eq!(headings(&mut b)[2..].to_vec(), names(&["vm-kil", "vm-str", "vm-ssa", "vm-not", "vm-demo"]));
    let asked = std::fs::read_to_string(far.dir.join("asked-herdr")).unwrap_or_default();
    assert!(!asked.contains("elias-macbook-pro-m1"), "{asked}");
    assert_eq!(asked.lines().count(), 5, "the machines that send no list are asked for herdr's: {asked}");
}
