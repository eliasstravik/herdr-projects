//! The rest of the old web-door tests: the web UI's PCs reach the board with only what it needs of each (never their
//! panes), with the PC the page is on, and are asked once for a moment, not with every look; and the Phone setup's
//! answer with the link through the door whole (its port kept, no command offered, no caching), while a link the web
//! UI found itself, Tailscale stopped, or an answer that is not one, are passed on as they are.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use sw::json::*;
use sw::webui::door::{handle, with_link, Upstream};
use sw::webui::pcs::Pc;

/// A stand-in for the web UI: its login, its PCs (each with its whole list of panes), and its Phone setup's answer.
fn fake_ui(seen: Arc<Mutex<Vec<String>>>) -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || for c in l.incoming() { let Ok(mut c) = c else { continue }; let seen = seen.clone(); std::thread::spawn(move || {
        let (head, _) = sw::webui::proxy::read_head(&mut c, 1 << 20).unwrap_or_default();
        let raw = String::from_utf8_lossy(&head).to_string();
        seen.lock().unwrap().push(raw.clone());
        let target = raw.split(' ').nth(1).unwrap_or("").to_string();
        let good = raw.contains("token=good");
        let body = if target.starts_with("/api/health") { json!({ "auth": if good { json!({ "authenticated": true, "role": "drive" }) } else { json!({ "authenticated": false }) } }) }
            else if target == "/api/machines" { json!({ "machines": [{ "id": "local", "name": "home", "kind": "local", "enabled": true, "state": "connected", "snapshot": { "panes": [] } }, { "id": "pc-1", "name": "far", "kind": "ssh", "target": { "destination": "me@far.example", "port": 2200 }, "enabled": true, "state": "connected", "snapshot": { "panes": [{ "pane_id": "w1:p1" }] } }] }) }
            else { json!({ "port": 7317, "tailscale": if target.contains("stopped") { json!({ "state": "stopped", "dns_name": null, "serving_url": null, "serve_command": null, "serve_url": null }) } else { json!({ "state": "running", "dns_name": "box.ts.net", "serving_url": if target.contains("has") { json!("https://box.ts.net:8443") } else { Value::Null }, "serve_command": "tailscale serve --bg --https=8443 http://127.0.0.1:7317", "serve_url": "https://box.ts.net:8443" }) } }) };
        let b = body.to_string();
        let _ = write!(c, "HTTP/1.1 200 OK\r\ncontent-type: application/json; charset=utf-8\r\ncache-control: no-store\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{b}", b.len());
    }); });
    port
}
struct Up { port: u16, link: String }
impl Upstream for Up { fn to(&self) -> Option<(String, u16)> { Some(("127.0.0.1".into(), self.port)) } fn link(&self) -> String { self.link.clone() } fn down(&self) {} }
fn ask(up: &Up, req: &str) -> (u16, String, String) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let req = req.to_string();
    let client = std::thread::spawn(move || { let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap(); s.write_all(req.as_bytes()).unwrap(); let mut out = vec![]; let _ = s.read_to_end(&mut out); String::from_utf8_lossy(&out).into_owned() });
    let (c, _) = l.accept().unwrap();
    handle(c, up);
    let out = client.join().unwrap();
    let (head, body) = out.split_once("\r\n\r\n").unwrap_or((&out, ""));
    (head.split(' ').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0), head.to_string(), body.to_string())
}
fn get(path: &str, headers: &str) -> String { format!("GET {path} HTTP/1.1\r\nhost: 127.0.0.1\r\n{headers}\r\n") }
const PAGE: &str = "cookie: token=good\r\ncontent-type: application/json\r\nx-herdr-projects: 1\r\nsec-fetch-site: same-origin\r\n";

#[test]
fn the_web_uis_pcs_reach_the_board_with_only_what_it_needs_with_the_pc_the_page_is_on_and_are_asked_once_for_a_moment() {
    let w = world();
    w.add("A task");
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str() }).to_string()).unwrap();
    let seen = Arc::new(Mutex::new(vec![]));
    let up = Up { port: fake_ui(seen.clone()), link: String::new() };
    let machines = || seen.lock().unwrap().iter().filter(|r| r.starts_with("GET /api/machines")).count();
    // a look of the page's own needs none
    assert_eq!(ask(&up, &get("/herdr-projects/api/board?client=abc", "cookie: token=good\r\n")).0, 200);
    assert_eq!(machines(), 0);
    let body = r#"{"id":"allDone"}"#;
    assert_eq!(ask(&up, &format!("POST /herdr-projects/api/press?client=abc HTTP/1.1\r\nhost: 127.0.0.1\r\ncontent-length: {}\r\n{PAGE}\r\n{body}", body.len())).0, 200);
    let far = Pc { id: "pc-1".into(), name: "far".into(), local: false, profile: String::new(), to: "me@far.example".into(), port: 2200, session: String::new(), connected: true, state: "connected".into(), panes: Some(vec!["w1:p1".into()]) }; // its panes by their ids alone
    {
        let c = sw::webui::web::client("abc"); let g = c.lock().unwrap();
        let pcs = g.s.web.pcs.clone().expect("the web UI's PCs were passed on");
        assert_eq!(pcs.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), vec!["local", "pc-1"]);
        assert_eq!(pcs[1], far, "only what the board needs of each");
        assert!(g.s.web.at.as_deref().unwrap_or("").is_empty(), "the page is on this machine");
    }
    assert!(seen.lock().unwrap().iter().any(|r| r.starts_with("GET /api/machines") && r.contains("token=good")), "asked with the caller's own cookie");
    // the page on another PC: the board is told which one, with the PCs it was kept a moment
    assert_eq!(ask(&up, &get("/herdr-projects/api/board?client=abc&machine=pc-1", "cookie: token=good\r\n")).0, 200);
    { let c = sw::webui::web::client("abc"); let g = c.lock().unwrap(); assert_eq!(g.s.web.at.as_deref(), Some("pc-1")); assert_eq!(g.s.web.pcs.as_ref().map(|l| l[1].clone()), Some(far)); }
    assert_eq!(machines(), 1, "kept for a moment, not asked with every look");
}

#[test]
fn the_sidebars_boards_come_through_the_door_with_the_web_uis_names_and_picking_one_is_a_look_not_a_change() {
    let w = world();
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str() }).to_string()).unwrap();
    let up = Up { port: fake_ui(Arc::new(Mutex::new(vec![]))), link: String::new() };
    assert_eq!(ask(&up, &get("/herdr-projects/api/boards?client=side", "")).0, 401, "not logged in, nothing of the board");
    let (st, _, body) = ask(&up, &get("/herdr-projects/api/boards?client=side", "cookie: token=good\r\n"));
    let j: Value = serde_json::from_str(&body).unwrap();
    assert_eq!((st, j.at("boards")[0].s("name")), (200, "home"), "this machine as the web UI names it, alone (no All machines with no other one): {body}");
    let pick = r#"{"to":""}"#;
    assert_eq!(ask(&up, &format!("POST /herdr-projects/api/scope?client=side HTTP/1.1\r\nhost: 127.0.0.1\r\ncontent-length: {}\r\n{PAGE}\r\n{pick}", pick.len())).0, 200);
    assert!(sw::webui::door::LOOKS.contains(&"scope"), "a device that only watches may pick a board");
}

#[test]
fn the_phone_setup_gets_the_link_through_the_door_whole_and_what_the_web_ui_says_itself_goes_on_as_it_is() {
    let seen = Arc::new(Mutex::new(vec![]));
    let port = fake_ui(seen);
    let access = |link: &str, q: &str| { let (st, head, body) = ask(&Up { port, link: link.into() }, &get(&format!("/api/access{q}"), "cookie: token=good\r\naccept-encoding: gzip\r\n")); assert_eq!(st, 200); (serde_json::from_str::<Value>(&body).unwrap(), head) };
    // no link goes through the front door: the web UI's answer is passed on as it is
    let (j, _) = access("", "");
    assert_eq!(j.at("tailscale"), &json!({ "state": "running", "dns_name": "box.ts.net", "serving_url": null, "serve_command": "tailscale serve --bg --https=8443 http://127.0.0.1:7317", "serve_url": "https://box.ts.net:8443" }));
    // its link goes through the front door: the answer names the link it has, and offers none
    let (j, head) = access("https://box.ts.net", "");
    assert_eq!(j, json!({ "port": 7317, "tailscale": { "state": "running", "dns_name": "box.ts.net", "serving_url": "https://box.ts.net", "serve_command": null, "serve_url": null } }));
    assert!(head.to_lowercase().contains("cache-control: no-store"), "{head}");
    // a link the web UI finds itself, and Tailscale not running, are its own to say
    assert_eq!(access("https://box.ts.net", "?has").0.at("tailscale").s("serving_url"), "https://box.ts.net:8443");
    let (j, _) = access("https://box.ts.net", "?stopped");
    assert_eq!((j.at("tailscale").s("state"), j.at("tailscale").at("serving_url")), ("stopped", &Value::Null));
    assert_eq!(with_link("not json", "https://box.ts.net"), None); assert_eq!(with_link("{}", "https://box.ts.net"), None);
}
