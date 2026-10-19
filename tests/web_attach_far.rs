//! A file attached in the web board's chat box for another machine's agent or coordinator reaches that machine: through
//! the herdr web UI's own way to that PC (asked by the front door with the caller's own headers, kept beside the pane
//! there), whose path is answered; for a heading's coordinator, and for the Coordinator of the machine whose board
//! shows; and where the web UI cannot (no PC, not connected, no pane known, or it says why not), over the board's own
//! SSH link, or the box is told plainly that it could not be sent there.
use crate::common;
use crate::far;
use common::*;
use far::Far;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use sw::json::*;
use sw::webui::attach::far::WebUi;
use sw::webui::door::{handle, Upstream};
use sw::webui::pcs::Pc;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nnot really a picture";
const THERE: &str = "/home/me/far/.herdr-web-ui/paste-1.png";
fn file(to: Value) -> Value { json!({ "name": "shot.png", "type": "image/png", "data": sw::util::base64(PNG), "to": to }) }
/// vm-a's board: one task with an agent in pane w1:p1, its Coordinator in w9:p1, its machine coordinator in w8:p1.
fn far_world() -> (World, Far) {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-a" }), ..Default::default() });
    let far = Far::new(&w.base);
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str() }).to_string()).unwrap();
    let mut task = far::theirs(1);
    task["link"] = json!({ "pane": "w1:p1" });
    far.answers("vm-a", "list", &json!({ "host": "vm-a", "version": "0.2.0", "tasks": [task],
        "coordinator": { "state": "idle", "talk": [], "pane": "w9:p1" }, "coordinators": { "machine": { "state": "idle", "talk": [], "pane": "w8:p1" } } }));
    (w, far)
}
/// That browser's board, every machine shown and vm-a's answer in.
fn every_machine(client: &str) {
    sw::webui::web::call(client, "press", json!({ "id": "scope", "role": "drive" }), "", None).unwrap();
    far::answered(client, 1);
}
fn pc(connected: bool) -> Pc { Pc { id: "pc-9".into(), name: "vm-a".into(), to: "vm-a".into(), connected, state: if connected { "connected" } else { "offline" }.into(), ..Default::default() } }
/// A stand-in for the web UI the front door reaches: its PCs, and what it answers a file.
struct Ui { pcs: Option<Vec<Pc>>, says: (u16, Value), asked: RefCell<Vec<(String, Value)>> }
impl WebUi for Ui {
    fn pcs(&self) -> Option<Vec<Pc>> { self.pcs.clone() }
    fn post(&self, path: &str, body: &Value) -> Option<(u16, Value)> { self.asked.borrow_mut().push((path.into(), body.clone())); Some(self.says.clone()) }
}
fn ui(pcs: Option<Vec<Pc>>, says: (u16, Value)) -> Ui { Ui { pcs, says, asked: RefCell::new(vec![]) } }
fn kept_there() -> (u16, Value) { (200, json!({ "ok": true, "path": THERE })) }
fn attach(to: Value, ui: &Ui) -> Result<Value, String> { sw::webui::web::call_with("t", "attach", { let mut a = file(to); a["role"] = json!("drive"); a }, "", None, Some(ui)) }

/// The web UI: its login, its PCs, and a file for a pane of PC pc-9 (head and body written down).
fn fake_ui(seen: Arc<Mutex<Vec<(String, String)>>>) -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || for c in l.incoming() { let Ok(mut c) = c else { continue }; let seen = seen.clone(); std::thread::spawn(move || {
        let (head, mut body) = sw::webui::proxy::read_head(&mut c, 1 << 20).unwrap_or_default();
        let raw = String::from_utf8_lossy(&head).to_string();
        let len: usize = raw.lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap_or(0))).unwrap_or(0);
        while body.len() < len { let mut chunk = vec![0u8; len - body.len()]; match c.read(&mut chunk) { Ok(0) | Err(_) => break, Ok(n) => body.extend_from_slice(&chunk[..n]) } }
        seen.lock().unwrap().push((raw.clone(), String::from_utf8_lossy(&body).to_string()));
        let target = raw.split(' ').nth(1).unwrap_or("").to_string();
        let answer = if target.starts_with("/api/health") { json!({ "auth": { "authenticated": true, "role": "drive" } }) }
            else if target == "/api/machines" { json!({ "machines": [{ "id": "local", "kind": "local", "state": "connected" }, { "id": "pc-9", "name": "vm-a", "kind": "ssh", "target": { "destination": "vm-a" }, "enabled": true, "state": "connected" }] }) }
            else { json!({ "ok": true, "path": THERE }) };
        let b = answer.to_string();
        let _ = write!(c, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{b}", b.len());
    }); });
    port
}
struct Up(u16);
impl Upstream for Up { fn to(&self) -> Option<(String, u16)> { Some(("127.0.0.1".into(), self.0)) } fn link(&self) -> String { String::new() } fn down(&self) {} }
fn through_door(up: &Up, path: &str, body: &str) -> (u16, String) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let req = format!("POST {path} HTTP/1.1\r\nhost: 127.0.0.1\r\ncookie: token=good\r\ncontent-type: application/json\r\nx-herdr-projects: 1\r\nsec-fetch-site: same-origin\r\ncontent-length: {}\r\n\r\n{body}", body.len());
    let client = std::thread::spawn(move || { let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap(); s.write_all(req.as_bytes()).unwrap(); let mut out = vec![]; let _ = s.read_to_end(&mut out); String::from_utf8_lossy(&out).into_owned() });
    let (c, _) = l.accept().unwrap();
    handle(c, up);
    let out = client.join().unwrap();
    let (head, body) = out.split_once("\r\n\r\n").unwrap_or((&out, ""));
    (head.split(' ').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0), body.to_string())
}

#[test]
fn a_file_for_another_machines_agent_goes_through_the_web_ui_to_its_pane_there_with_the_callers_own_headers() {
    let (_w, far) = far_world();
    every_machine("door");
    let seen = Arc::new(Mutex::new(vec![]));
    let up = Up(fake_ui(seen.clone()));
    let (status, body) = through_door(&up, "/herdr-projects/api/attach?client=door", &file(json!({ "id": "vm-a:far/1", "reply": true, "scope": "*" })).to_string());
    let out: Value = serde_json::from_str(&body).unwrap();
    assert_eq!((status, out.s("path")), (200, THERE), "the path on that machine, for the box to name: {body}");
    let seen = seen.lock().unwrap().clone();
    let (head, sent) = seen.iter().find(|(h, _)| h.starts_with("POST /api/machines/pc-9/pane/image ")).cloned().expect("the web UI was asked to keep it beside that pane");
    let head = head.to_lowercase();
    assert!(head.contains("cookie: token=good") && head.contains("x-herdr-machine: 1") && head.contains("content-type: application/json"), "{head}");
    let sent: Value = serde_json::from_str(&sent).unwrap();
    assert_eq!((sent.s("pane_id"), sent.s("content_type"), sent.s("name")), ("w1:p1", "image/png", "shot.png"));
    assert_eq!(sw::util::unbase64(sent.s("data_base64")).unwrap(), PNG);
    assert!(!far.asked().iter().any(|a| a.1 == "keep"), "nothing over the board's own link");
}

#[test]
fn a_file_for_another_machines_coordinator_by_its_heading_or_by_its_board_goes_beside_that_coordinator() {
    let (_w, _far) = far_world();
    every_machine("t");
    let pane = |to: Value| { let u = ui(Some(vec![pc(true)]), kept_there()); assert_eq!(attach(to, &u).unwrap().s("path"), THERE); let a = u.asked.borrow()[0].clone(); assert_eq!(a.0, "/api/machines/pc-9/pane/image"); a.1.t("pane_id") };
    // vm-a's heading: its machine coordinator
    assert_eq!(pane(json!({ "id": "head:machine\tvm-a", "reply": true, "scope": "*" })), "w8:p1");
    // vm-a's board: its Coordinator, also when the box said nobody (the board's own scope then, never this machine's)
    sw::webui::web::call("t", "scope", json!({ "to": "vm-a", "role": "drive" }), "", None).unwrap();
    assert_eq!(pane(json!({ "id": "", "reply": false, "scope": "vm-a" })), "w9:p1");
    assert_eq!(pane(Value::Null), "w9:p1");
}

#[test]
fn where_the_web_ui_cannot_the_file_goes_over_the_boards_own_link_or_why_it_could_not_is_said() {
    let (_w, far) = far_world();
    far.answers("vm-a", "keep", &json!({ "ok": true, "path": "/home/me/far/.herdr-projects/files/paste-2.png" }));
    every_machine("t");
    let task = json!({ "id": "vm-a:far/1", "reply": true, "scope": "*" });
    // the web UI has no PC for vm-a, has it but not connected, or could not say: no file goes to the web UI
    for pcs in [Some(vec![]), Some(vec![pc(false)]), None] {
        let u = ui(pcs, kept_there());
        let out = attach(task.clone(), &u);
        assert!(u.asked.borrow().is_empty());
        over_ssh(&far, out, "far/1", "");
    }
    // the web UI says why not (the pane is gone): that is said too, if the other way fails as well
    let u = ui(Some(vec![pc(true)]), (404, json!({ "error": { "code": "pane_not_found", "message": "pane w1:p1 not found" } })));
    over_ssh(&far, attach(task, &u), "far/1", "the web UI said: pane w1:p1 not found");
    // a machine on an older version sends no pane of its coordinator: the board's own link only
    far.answers("vm-a", "list", &json!({ "host": "vm-a", "version": "0.1.0", "tasks": [], "coordinator": { "state": "idle", "talk": [] } }));
    far::ask_again("t", 1);
    let u = ui(Some(vec![pc(true)]), kept_there());
    let out = attach(json!({ "id": "", "reply": false, "scope": "vm-a" }), &u);
    assert!(u.asked.borrow().is_empty());
    over_ssh(&far, out, "", "");
}
/// Sent over the board's own link (`remote keep`, for that task or that coordinator) and its path there answered; or,
/// while that cannot be, a plain word that it could not be sent to vm-a (with what the web UI said, if it said).
fn over_ssh(far: &Far, out: Result<Value, String>, id: &str, web: &str) {
    match out {
        Ok(v) => {
            assert_eq!(v.s("path"), "/home/me/far/.herdr-projects/files/paste-2.png");
            let k = far.asked().into_iter().filter(|a| a.1 == "keep").last().expect("asked to keep it");
            assert_eq!((k.0.as_str(), k.2.s("id")), ("vm-a", id));
        }
        Err(e) => assert!(e.starts_with("The file could not be sent to vm-a: ") && e.contains(web), "{e}"),
    }
}
