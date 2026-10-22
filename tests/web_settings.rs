//! The web board's settings page: every setting of the open task with its plain label and what it means, in the
//! terminal page's order; changed and put back there; the task's own panel saying the same in words, its settings
//! above the long texts; a refused value refused in the board's words; read-only for a watcher and an archived task.
use crate::common;
use common::board::{web, web_as};
use common::*;
use serde_json::{json, Value};
use sw::json::*;

fn row(s: &Value, name: &str) -> Value { s.at("settings").arr("rows").iter().find(|x| x.s("name") == name).cloned().unwrap_or_else(|| panic!("no row {name}")) }
fn set(w: &World, id: &str, name: &str, value: Value) -> Value { web(w, "set", json!({ "id": id, "name": name, "value": value })) }
fn rows_of(s: &Value) -> Vec<Value> { s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").to_vec()).collect() }

#[test]
fn every_setting_of_the_open_task_with_its_label_and_meaning_changed_and_put_back_there() {
    let w = world();
    let t = w.add("Look into the logs");
    assert!(web(&w, "board", json!({})).at("settings").is_null(), "no task open: nothing to set");
    let s = web(&w, "pick", json!({ "id": t, "open": true }));
    assert_eq!((s.at("settings").s("id"), s.at("settings").s("whose"), s.at("settings").at("canSet")), (t.as_str(), "this task", &json!(true)));
    // the terminal page's rows, in its order
    let want: Vec<String> = { let c = sw::webui::web::client("test"); let mut g = c.lock().unwrap(); let was = g.s.editing.replace(sw::board::state::Editing { id: t.clone(), ..Default::default() }); let r = sw::board::update::edit::rows(&g.s); g.s.editing = was; r.iter().map(|x| x.t("name")).collect() };
    assert_eq!(s.at("settings").arr("rows").iter().map(|x| x.t("name")).collect::<Vec<_>>(), want);
    assert_eq!(row(&s, "research.only"), json!({ "name": "research.only", "label": "Research only", "means": "its agent changes nothing in the repo; the result is a report, and nothing is pushed, opened or merged", "own": false, "value": "off", "from": "",
        "words": [{ "value": "true", "say": "on", "now": false }, { "value": "false", "say": "off", "now": true }], "hint": "" }));
    assert!(row(&s, "review.rounds").at("words").is_null()); assert!(row(&s, "review.rounds").s("hint").contains("a whole number, 0 or more"));
    // The two limits are there like every setting: the plain label, what the value may be, and what it means.
    let lt = row(&s, "limit.time");
    assert_eq!((lt.s("label"), lt.s("value"), lt.at("words")), ("Longest run", "none", &Value::Null));
    assert!(lt.s("hint").starts_with("a time, such as 90s or 15m; none for none. Means: the longest one run of an agent may take"), "{}", lt.s("hint"));
    assert!(regex::Regex::new(r"^the longest one run of an agent may take.*the task waits for you").unwrap().is_match(lt.s("means")), "{}", lt.s("means"));
    let lk = row(&s, "limit.tokens");
    assert_eq!(lk.s("label"), "Most tokens"); assert!(lk.s("hint").starts_with("a count of tokens, such as 500k or 2m; none for none"), "{}", lk.s("hint")); assert!(lk.s("means").starts_with("the most tokens the task may use"));
    // A refused value: the board's words say why (the Rust settings page keeps it in the box to be put right, and says so
    // as its message: no error is thrown back at the page).
    let s = set(&w, &t, "limit.time", json!("2 hours"));
    assert!(s.at("msg").s("text").contains("Did you mean 2h?"), "{}", s.at("msg")); assert_eq!(row(&s, "limit.time").s("value"), "none");
    let s = set(&w, &t, "limit.time", json!("2h")); assert_eq!((row(&s, "limit.time").s("value"), row(&s, "limit.time").at("own")), ("2h", &json!(true)));
    let s = set(&w, &t, "limit.time", Value::Null); assert_eq!(row(&s, "limit.time").s("value"), "none");
    // Changed from the page: the task's own, the label on its row follows, and the board says what it is now.
    let s = set(&w, &t, "research.only", json!("true"));
    assert_eq!(task(&t).at("settings"), &json!({ "research.only": true }));
    assert_eq!((row(&s, "research.only").s("value"), row(&s, "research.only").at("own"), row(&s, "research.only").s("from")), ("on", &json!(true), ""));
    assert!(!s.at("msg").s("text").starts_with("Research only"), "no confirmation: the row says it: {}", s.at("msg"));
    assert!(rows_of(&s).iter().find(|x| x.s("id") == t).unwrap().s("title").contains("Look into the logs [research]"));
    // The task's own panel says the same, in words: every setting it runs with, a `*` after a value set for this task
    // only. No value says where it comes from.
    {
        let facts: Vec<Value> = s.at("panel").arr("parts").iter().find(|p| p.s("head") == "Settings").unwrap().arr("items").iter().filter(|x| x.s("k") == "fact").cloned().collect();
        let said = |label: &str| facts.iter().find(|x| x.s("label") == label).map(|x| x.arr("lines").iter().flat_map(|l| l.as_array().cloned().unwrap_or_default()).map(|p| p.t("t")).collect::<String>()).unwrap_or_default();
        assert_eq!(facts.iter().map(|x| x.t("label")).filter(|k| !["Project", "Worktree", "Branch", "Note"].contains(&k.as_str())).collect::<Vec<_>>(), vec!["Agent", "Plan first", "Reviewer", "Review when", "Send back", "Check", "Research", "Workspace", "Also"]);
        assert_eq!(said("Research"), "only · no change to the repo *"); assert_eq!(said("Send back"), "major or worse · 3 rounds max");
        assert!(!s.at("panel").to_string().contains("as usual"));
    }
    // A long description never pushes them down the page: they stand right under the status, and under what the task needs from you.
    let heads = |hs: &[(&str, &str)]| sw::webui::model::panel::settings_up(hs.iter().map(|(h, c)| obj(json!({ "head": h, "color": c }))).collect()).iter().map(|p| p.t("head")).collect::<Vec<_>>();
    assert_eq!(heads(&[("Status", ""), ("Asks you", "red"), ("Description", ""), ("The agent said", ""), ("Settings", ""), ("Session", "")]), vec!["Status", "Asks you", "Settings", "Description", "The agent said", "Session"]);
    sw::store::tasks::patch(&t, |x| { x.put("description", (1..=80).map(|i| format!("Line {i} of a very long description.")).collect::<Vec<_>>().join("\n")); }).unwrap();
    let hs: Vec<String> = web(&w, "board", json!({})).at("panel").arr("parts").iter().map(|p| p.t("head")).collect();
    let at = |h: &str| hs.iter().position(|x| x == h).unwrap();
    assert!(at("Settings") < at("Description") && at("Settings") <= 2, "{hs:?}");
    // A typed value; a wrong one is refused in the board's words and changes nothing.
    let s = set(&w, &t, "review.rounds", json!("5")); assert_eq!((row(&s, "review.rounds").s("value"), task(&t).at("settings").at("review.rounds")), ("5", &json!(5)));
    let s = set(&w, &t, "review.rounds", json!("many")); assert!(s.at("msg").s("text").contains("review.rounds must be a whole number"), "{}", s.at("msg")); assert_eq!(task(&t).at("settings").at("review.rounds"), &json!(5));
    let s = set(&w, &t, "board.agents", json!("9")); assert!(s.at("msg").s("text").contains("board.agents is a board setting"), "{}", s.at("msg"));
    // Put back: what it would be without the task's own answer.
    let s = set(&w, &t, "research.only", Value::Null);
    assert_eq!(task(&t).at("settings"), &json!({ "review.rounds": 5 })); assert_eq!((row(&s, "research.only").s("value"), row(&s, "research.only").at("own")), ("off", &json!(false)));
    // A device that only watches changes nothing; nor does a change meant for a task that is no longer open.
    assert!(web_as(&w, "test", "set", json!({ "id": t, "name": "research.only", "value": "true", "role": "watch" })).unwrap_err().contains("can only watch"));
    let s = set(&w, "proj/999", "research.only", json!("true"));
    assert!(s.at("msg").s("text").contains("no longer the one that is open"), "{}", s.at("msg")); assert!(task(&t).at("settings").at("research.only").is_null());
    // Its key in More goes there; an archived task's settings are read-only.
    let s = web(&w, "press", json!({ "id": "settings" })); assert_eq!(s.at("settings").i("show"), 1); assert!(!s.at("panel").is_null());
    sw::actions::archive::archive(&t).unwrap();
    web(&w, "press", json!({ "id": "archiveView" }));
    assert_eq!(web(&w, "pick", json!({ "id": t, "open": true })).at("settings").at("canSet"), &json!(false));
}
