//! A task of another machine acted on from the command line, named by its machine first (`tell vm-b:web/7 <text>`):
//! the action goes to that machine's door (`do`), as the board's keys send it there, and its answer is the command's.
//! So a coordinator (its autopilot too) looks after the tasks it added on another machine as it does its own: `details`
//! reads there what its agent asks or why it failed.
use crate::err::Res;
use crate::json::*;

/// The commands that reach another machine's task, and the door's action for each.
const ACTIONS: [(&str, &str); 7] = [("tell", "tell"), ("retry", "retry"), ("stop", "stop"), ("archive", "archive"), ("approve", "approve"), ("delegate", "delegate"), ("details", "details")];
/// Some: the command named another machine's task, and this is what that machine answered.
/// `by_autopilot`: it runs in an autopilot turn, and that machine holds its yes to the rules of one (remote.rs: door).
pub fn far_command(cmd: &str, pos: &[String], flags: &Obj, by_autopilot: bool) -> Res<Option<(Value, String)>> {
    let Some((_, action)) = ACTIONS.iter().find(|(c, _)| *c == cmd) else { return Ok(None) };
    let Some((label, rid)) = pos.get(1).and_then(|r| crate::after::far::split(r)) else { return Ok(None) };
    if crate::after::circle::is_me(&label) { return Ok(None); }
    let Some(m) = super::machines().into_iter().find(|m| m.label.eq_ignore_ascii_case(&label)) else { return Ok(None) };
    let text = pos.iter().skip(2).cloned().collect::<Vec<_>>().join(" ");
    // An autopilot turn's yes is an action of its own there, `pilot-approve`, which only a machine that checks it knows:
    // one that does not answers that it has no such action, and nothing is approved.
    let action = if *action == "approve" && by_autopilot { "pilot-approve" } else { *action };
    let mut req = obj(json!({ "action": action, "id": rid, "text": text, "anyway": flags.b("anyway"), "autopilot": by_autopilot }));
    if ["delegate", "tell"].contains(&action) {
        let repo = rid.split('/').next().unwrap_or("").to_string();
        req.put("settings", crate::store::settings::lookup::to_send(&repo, &crate::store::settings::places::yours())); req.put("from", super::check::hostname());
    }
    match super::ask(&m, "do", Some(&Value::Object(req)), 30000) {
        Err(why) => crate::fail!("{}", super::why::unanswered(&m.label, &why)),
        // `details`: where it stands there, whole, as `details` says it of a task here.
        Ok(j) if j.b("ok") && action == "details" => { let d = j.at("details").clone(); let said = serde_json::to_string_pretty(&d).unwrap_or_default(); Ok(Some((d, said))) }
        Ok(j) if j.b("ok") => { let said = format!("{}: {}", m.label, j.s("msg")); Ok(Some((j, said))) }
        Ok(j) if j.s("msg").starts_with("Unknown action") => crate::fail!("{} runs an older version of herdr-projects that cannot {} its task from another machine: update it there first.", m.label, if action == "pilot-approve" { "check an autopilot turn's yes for" } else { action }),
        Ok(j) => crate::fail!("{}: {}", m.label, j.s("msg")),
    }
}
