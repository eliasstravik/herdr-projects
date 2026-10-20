//! Another machine's automations: what this machine sends of its own when a board elsewhere asks for its list, and
//! what it does when that board turns one on or off, fires it, removes it or changes one of its settings. An
//! automation never moves: it is changed on the machine that has it, as a task is (remote.rs).
use crate::automations::{fire::fire, list as l, show::{fired_words, firings, turned_words}, told::Cause};
use crate::err::Res;
use crate::json::*;
use crate::store::{automations::automations, settings::values::say, settings_list::entry_of};

/// The firings sent with each: what the asking board's panel lists.
const FIRINGS_SENT: usize = 20;
/// Each automation as it is saved, with what the asking side cannot read from here: its latest firings, newest first,
/// the settings that are its own, every work setting as its next task will have it here, and whether its tasks are
/// research only, worked out here, where its settings are.
pub fn automations_sent() -> Vec<Value> {
    automations().into_iter().map(|a| {
        let rows = l::setting_rows(&a);
        let sent: Vec<Value> = rows.iter().map(|x| json!({ "name": x.name, "value": x.value, "from": x.from, "own": x.own, "note": x.note })).collect();
        let own: Vec<Value> = rows.iter().filter(|x| x.own).map(|x| json!({ "name": x.name, "value": x.value, "own": true })).collect();
        let mut o = a.clone();
        o.put("research", l::research_only(&a));
        o.put("chat", l::chat_only(&a));
        o.put("shown", json!({ "firings": firings(&a).into_iter().take(FIRINGS_SENT).collect::<Vec<_>>(), "own": own, "settings": sent }));
        Value::Object(o)
    }).collect()
}
pub fn is_automation_action(action: &str) -> bool { ["auto-turn", "auto-run", "auto-remove", "auto-set"].contains(&action) }
/// One of the things a board does to an automation, answered as every action is: {ok, msg}.
pub fn automation_do(req: &Value) -> Res<Value> {
    let a = l::find(req.s("id"))?;
    match req.s("action") {
        "auto-turn" => Ok(json!({ "ok": true, "msg": turned_words(&l::turn(a.s("id"), *req.at("on") == json!(true))?).0 })),
        // One of its own settings, from the asking board's settings page: a value, or none to put the row back.
        "auto-set" => {
            let name = req.t("name");
            let r = if req.at("value").is_null() { l::unset_setting(a.s("id"), &name)? } else { l::set_setting(a.s("id"), &name, req.at("value"))? };
            let Some(r) = r else { return Ok(json!({ "ok": true, "msg": name })) };
            let label = entry_of(&r.name).map(|e| e.label.to_string()).unwrap_or_else(|| r.name.clone());
            Ok(json!({ "ok": true, "msg": crate::store::settings::show::saved_words(&label, say(&r.value).lines().next().unwrap_or(""), &r.from) }))
        }
        "auto-remove" => { l::remove(a.s("id"))?; Ok(json!({ "ok": true, "msg": format!("Removed the automation \"{}\".", a.s("name")) })) }
        _ => {
            if !a.b("on") { return Ok(json!({ "ok": false, "msg": format!("\"{}\" is off, and off it never fires.", a.s("name")) })); }
            let f = fire(a.s("id"), Cause::Run, crate::clock::now())?;
            Ok(json!({ "ok": f.b("task"), "msg": fired_words(&a, &f).0 }))
        }
    }
}
