//! A task of another machine acted on from the command line, named by its machine first (`tell vm-b:web/7 <text>`):
//! the action goes to that machine's door (`do`), as the board's keys send it there, and its answer is the command's.
//! So a coordinator (its autopilot too) looks after the tasks it added on another machine as it does its own.
use crate::err::Res;
use crate::json::*;

/// The commands that reach another machine's task, and the door's action for each.
const ACTIONS: [(&str, &str); 6] = [("tell", "tell"), ("retry", "retry"), ("stop", "stop"), ("archive", "archive"), ("approve", "approve"), ("delegate", "delegate")];
/// Some: the command named another machine's task, and this is what that machine answered.
pub fn far_command(cmd: &str, pos: &[String], flags: &Obj) -> Res<Option<(Value, String)>> {
    let Some((_, action)) = ACTIONS.iter().find(|(c, _)| *c == cmd) else { return Ok(None) };
    let Some((label, rid)) = pos.get(1).and_then(|r| crate::after::far::split(r)) else { return Ok(None) };
    if crate::after::circle::is_me(&label) { return Ok(None); }
    let Some(m) = super::machines().into_iter().find(|m| m.label.eq_ignore_ascii_case(&label)) else { return Ok(None) };
    let text = pos.iter().skip(2).cloned().collect::<Vec<_>>().join(" ");
    let mut req = obj(json!({ "action": action, "id": rid, "text": text, "anyway": flags.b("anyway") }));
    if ["delegate", "tell"].contains(action) {
        let repo = rid.split('/').next().unwrap_or("").to_string();
        req.put("settings", crate::store::settings::lookup::to_send(&repo, &crate::store::settings::places::yours())); req.put("from", super::check::hostname());
    }
    match super::ask(&m, "do", Some(&Value::Object(req)), 30000) {
        Err(why) => crate::fail!("{}", super::why::unanswered(&m.label, &why)),
        Ok(j) if j.b("ok") => { let said = format!("{}: {}", m.label, j.s("msg")); Ok(Some((j, said))) }
        Ok(j) => crate::fail!("{}: {}", m.label, j.s("msg")),
    }
}
