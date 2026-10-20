//! Reviews returned to the existing visible orchestrator session, with the same verdict contract and correction gate.
use super::review::Reviewed;
use super::review_brief::{review_brief, parse_verdict, Brief};
use super::shared::{review_until, timed_out};
use crate::coordinator::{co::Co, requests, state};
use crate::err::Res;
use crate::gate;
use crate::json::*;
use crate::runner::{Guard, is_aborted};
use crate::store::files::artifacts_dir;
use crate::store::repos::cwd_of;
use crate::store::reviews::{keep_saved_review, review_history, review_round};
use crate::store::tasks::get;

pub fn run(id: &str, g: &mut Guard, text: &str, change: Option<&Value>, to_land: bool, ch: &Obj, check: &str) -> Res<Reviewed> {
    let t = g.set(|t| {
        keep_saved_review(t);
        t.put("reviewRound", t.i("reviewRound") + 1); t.put("stage", "reviewing");
        t.put("msg", "Der Orchestrator prüft das Ergebnis"); t.put("reviewAt", crate::clock::now()); t.put("waiting", Value::Null);
    })?;
    g.allow(&["reviewneeds"]);
    let co = crate::mission_control::coordinator_for_task(&t, t.at("link").s("socket")).unwrap_or_else(|| Co::top(t.at("link").s("socket")));
    let folder = cwd_of(&t);
    let dir = crate::flow::task::review_dir(&t);
    let verdict = dir.join("verdict.json");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let history: Vec<Value> = review_history(&t).into_iter().filter(gate::is_tagged).collect();
    let earlier = if t.i("reviewRounds") > 0 { history.last().map(|h| h.arr("findings").to_vec()) } else { None };
    let used: Vec<String> = history.iter().flat_map(|h| h.arr("findings").iter().map(|f| f.t("id"))).collect();
    let body = review_brief(&t, &Brief { text, change, to_land, ch, earlier: earlier.as_deref(), check });
    let prompt = format!("[ORCHESTRATOR-REVIEW]\nDie Teilaufgabe {} ist fertig. Prüfe sie jetzt selbst in dieser laufenden Sitzung. Lege für dieses Review keinen weiteren Task oder Agenten an. Arbeitsordner: {}. Du arbeitest weiter in deinem eigenen Terminal; benutze für Befehle im Arbeitsstand diesen vollständigen Pfad. Verändere den Arbeitsstand nicht. Schreibe dein Urteil wie unten verlangt nach {}. Melde dem Menschen Ergebnis und eventuelle Fehler.\n\n{body}", t.s("id"), folder.display(), verdict.display());
    let queued = requests::send(&folder, &prompt, false, id, &co)?;
    let _pending = Pending { co: co.clone(), id: queued.arr("queue").last().map(|m| m.t("id")).unwrap_or_default() };
    if crate::runner::inline() { crate::coordinator::pump::pump(&co); }
    let until = review_until(ch);
    let head = if to_land { change.map(|c| c.t("head")).unwrap_or_default() } else { String::new() };
    let mut reason = String::new();
    let mut parsed = None;
    loop {
        if let Err(e) = g.check() {
            if is_aborted(&e) { return Ok(Reviewed { aborted: true, ..Default::default() }); }
            return Err(e);
        }
        if until.map(|end| crate::clock::now() >= end).unwrap_or(false) { reason = timed_out("Das Orchestrator-Review", ch); break; }
        if let Ok(raw) = std::fs::read_to_string(&verdict) {
            match parse_verdict(&raw, &head, earlier.as_deref(), &used) {
                Ok(Some(review)) => { parsed = Some(review); break; }
                Ok(None) => reason = "Das Urteil gehört zu einem anderen Commit; das aktuelle Ergebnis wartet weiter auf Review.".into(),
                Err(e) => reason = format!("Das Review-Urteil ist noch ungültig: {}", e.msg),
            }
        }
        if !reason.is_empty() && get(id).map(|t| t.s("msg") != reason).unwrap_or(false) {
            g.set(|t| { t.put("msg", reason.clone()); })?;
        }
        let c = state::get(&co).unwrap_or_default();
        if c.b("error") { reason = format!("Orchestrator: {}", c.s("error")); break; }
        let api = crate::herdr::Herdr::new(Some(co.socket.clone()));
        let a = api.agent_get(c.s("pane")).ok().flatten();
        let blocked = a.as_ref().map(|a| a.s("agent_status") == "blocked").unwrap_or(false);
        let current = get(id).unwrap_or_default();
        if blocked && current.s("stage") == "reviewing" {
            g.set(|t| { t.put("stage", "reviewneeds"); t.put("msg", "Der Orchestrator wartet auf eine Antwort im sichtbaren Terminal"); })?;
            g.allow(&["reviewing"]);
        } else if !blocked && current.s("stage") == "reviewneeds" {
            g.set(|t| { t.put("stage", "reviewing"); t.put("msg", "Der Orchestrator prüft das Ergebnis"); })?;
            g.allow(&["reviewneeds"]);
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    let kind = state::get(&co).map(|c| c.at("run").t("kind")).unwrap_or_default();
    let by = format!("Orchestrator ({kind})");
    let Some(mut rv) = parsed else { return Ok(Reviewed { error: reason, by, ..Default::default() }) };
    gate::stamp(&mut rv, ch.s("review.mustFix")); rv.put("by", by.clone());
    let art = artifacts_dir(id);
    std::fs::create_dir_all(&art)?;
    crate::util::copy_dir(&dir, &art)?;
    let t = get(id).unwrap_or(t);
    let ctx = json!({ "id": id, "round": review_round(&t), "title": t.at("title"), "description": t.at("description"), "summary": text,
        "stat": change.map(|c| c.at("stat").clone()), "head": change.map(|c| c.at("head").clone()), "pr": t.at("pr") });
    let artifacts = crate::review_page::publish_review(ch.b("review.evidence"), &art, rv.at("listed"), &Value::Object(rv.clone()), &t, &ctx).unwrap_or_default();
    rv.put("artifacts", artifacts); rv.del("listed");
    // The orchestrator pane remains open after reviews and failures.
    Ok(Reviewed { review: Some(rv), by, ..Default::default() })
}

struct Pending { co: Co, id: String }
impl Drop for Pending {
    fn drop(&mut self) {
        if self.id.is_empty() { return; }
        let _ = state::change(&self.co, |c| {
            let queue: Vec<Value> = c.arr("queue").iter().filter(|m| m.s("id") != self.id).cloned().collect();
            if queue.len() == c.arr("queue").len() { return false; }
            c.put("queue", queue); true
        });
    }
}
