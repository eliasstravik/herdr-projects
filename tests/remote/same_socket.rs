//! The same herdr socket reached by another path (a config folder through a symlink, from another machine's login):
//! the task is still this session's, and what is asked of it is done. A socket of another session is still refused.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;

fn door(what: &str, req: Value) -> Value { sw::remote::door(what, &req).unwrap_or_else(|e| json!({ "ok": false, "msg": e.msg })) }
fn here_is(socket: &std::path::Path) { unsafe { std::env::set_var("HERDR_SOCKET_PATH", socket) } }

#[test]
fn a_task_answers_through_another_path_to_its_own_socket_and_not_to_another_session() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Remote task");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "working");
    std::os::unix::fs::symlink(&w.base, w.base.join("config")).unwrap();
    here_is(&w.base.join("config/herdr.sock"));
    let row = door("list", json!({})).arr("tasks").iter().find(|t| t.s("id") == id).cloned().unwrap();
    assert_eq!(row.s("state"), "working", "{row}");
    let told = door("do", json!({ "action": "tell", "id": id, "text": "Also add a test." }));
    assert_eq!(told.at("ok"), &json!(true), "{told}");
    assert!(prompts(&w, task(&id).at("link").s("pane")).iter().any(|p| p.contains("Also add a test.")));
    std::fs::create_dir_all(w.base.join("sessions/two")).unwrap();
    here_is(&w.base.join("sessions/two/herdr.sock"));
    let refused = door("do", json!({ "action": "tell", "id": id, "text": "Not here." }));
    assert!(refused.s("msg").contains("lives in herdr session default"), "{refused}");
}
