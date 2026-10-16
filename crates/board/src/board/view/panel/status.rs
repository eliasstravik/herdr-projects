//! The top of the task panel: the title, where the task stands under its Status headline, and what it needs from you.
use super::kit::Kit;
use super::Ctx;
use crate::board::keys::key_name;
use crate::json::*;
use crate::limits::run::{stopped_at_limit, used_say};
use crate::limits::shown::{time_line, tokens_line, NEAR};
use crate::limits::spent::can_count;
use crate::shown::ink::{Ink, Inked};
use crate::shown::review_words::{review_note, round_words};
use crate::shown::states::{failed_review, reviewer_word, reviewing, standing};
use crate::shown::words::{age, bar, clean, merged, pr_number, pr_color};
use crate::status_words::{needs, review_tally};
use crate::store::retries::waiting_of;

/// The facts under Status, in the order they are drawn.
pub const ORDER: [&str; 12] = ["head", "state", "after", "progress", "now", "time", "tokens", "who", "over", "made", "change", "next"];
fn same(a: &str, b: &str) -> bool { a.trim().to_lowercase() == b.trim().to_lowercase() }
fn asks(kind: &str) -> String { needs(kind).map(|w| { let mut c = w.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() }).unwrap_or_else(|| "Needs you".into()) }
/// The status part's facts by name, redrawn when some go.
pub struct Top { pub rows: Vec<(&'static str, Vec<Inked>)> }
impl Top {
    pub fn lines(&self) -> Vec<Inked> { ORDER.iter().flat_map(|k| self.rows.iter().find(|(n, _)| n == k).map(|(_, l)| l.clone()).unwrap_or_default()).collect() }
    pub fn drop(&mut self, keys: &[&str]) { self.rows.retain(|(k, _)| !keys.contains(k)); }
    pub fn get(&self, k: &str) -> Option<&Vec<Inked>> { self.rows.iter().find(|(n, _)| *n == k).map(|(_, l)| l) }
    pub fn set(&mut self, k: &'static str, v: Vec<Inked>) { if let Some(x) = self.rows.iter_mut().find(|(n, _)| *n == k) { x.1 = v; } }
}
/// Returns the title's part, the status part's facts, and what the part about what it needs from you printed.
pub fn status(k: &mut Kit, c: &Ctx, r: &Obj) -> (usize, Top, usize, String) {
    let ink = crate::shown::states::status_ink(r);
    // The title, with the task's number before it, greyed as on its row.
    let tag = c.tag.clone();
    let words = format!("{}{}", if tag.is_empty() { String::new() } else { format!("{tag} ") }, if r.b("title") { r.s("title") } else { "(no title)" });
    let title_lines: Vec<Inked> = k.wrap(&words).into_iter().enumerate().map(|(n, l)| { let t = l.text(); if n == 0 && !tag.is_empty() && t.starts_with(&tag) { Kit::dim(tag.clone()).add(Inked::plain(t[tag.len()..].to_string()).bold()) } else { Inked::plain(t).bold() } }).collect();
    let title = k.part(title_lines);
    let working = r.s("state") == "working";
    let when = format!("{}{}{}", if working { "for " } else { "" }, age(c.now - if r.b("at") { r.i("at") } else { c.now }), if working { "" } else { " ago" });
    let mut rows: Vec<(&'static str, Vec<Inked>)> = vec![("head", vec![k.head("Status", None)])];
    let under = crate::shown::states::under_review(r);
    let stand = if r.b("archivedAt") { "archived".to_string() } else if under { reviewer_word(r).into() } else { standing(r) };
    let rw = round_words(r);
    rows.push(("state", k.fact("State", k.flow(vec![c.mark.clone().add(Inked::plain(" ")).add(Inked::of(stand.clone(), ink).bold()), if (reviewing(r) || r.s("stage") == "approval") && !rw.is_empty() { Inked::of(rw, ink) } else { Inked::new() }, Kit::dim(when)], None, " · "))));
    // What it waits for, or waited for, before it starts (after.rs): each task, and whether it is done.
    if let Some((label, line)) = crate::after::panel_line(r) { rows.push(("after", k.fact(label, Kit::wrap_at(&line, k.room).into_iter().map(|l| l.recolor(Ink::Soft)).collect()))); }
    let has_eta = r.at("eta").is_number();
    let eta = if has_eta { format!("{}m left", r.t("eta")) } else { String::new() };
    if r.at("progress").is_number() && r.s("state") != "done" {
        let pct = format!(" {}%", r.t("progress"));
        let beside = k.room as i64 - pct.len() as i64 - if has_eta { eta.len() as i64 + 3 } else { 0 };
        let n = (if beside >= 8 { beside } else { k.room as i64 - pct.len() as i64 }).clamp(1, 24) as usize;
        rows.push(("progress", k.fact("Progress", k.flow(vec![Inked::of(format!("{}{pct}", bar(r.f("progress"), n)), ink), if has_eta { Inked::of(eta.clone(), ink) } else { Inked::new() }], None, " · "))));
    } else if has_eta && r.s("state") != "done" { rows.push(("progress", k.fact("Progress", vec![Inked::of(eta.clone(), ink)]))); }
    let note = review_note(r);
    let failure = c.failure.clone();
    // What you just did, under way (update/pending.rs): said at once, before anything of it can be read.
    let p = r.at("pending");
    if p.b("word") { let ink = if p.b("failed") { Ink::Red } else { Ink::Yellow }; rows.push(("now", k.fact("Now", Kit::wrap_at(&p.t("word"), k.room).into_iter().map(|l| l.recolor(ink)).collect()))); }
    else if working && !note.is_empty() && !same(&note, &stand) && !(failure.is_some() && waiting_of(r).map(|w| w.s("who") == "agent").unwrap_or(false)) { rows.push(("now", k.fact_text("Now", &note))); }
    // What this run and the task have used against each limit that is set, in amber from near it on.
    let lim = r.at("limits");
    let near = |x: &Value, of: f64| x.f("used") >= NEAR * of;
    if lim.at("time").is_object() { let x = lim.at("time"); let l = Inked::of(time_line(x), if near(x, crate::store::settings::values::ms(x.s("limit")) as f64) { Ink::Peach } else { Ink::Soft }); rows.push(("time", k.fact("Time", vec![l]))); }
    if lim.at("tokens").is_object() {
        let x = lim.at("tokens");
        let counted = !x.b("kind") || can_count(x.s("kind"));
        let ink2 = if counted && near(x, crate::store::settings::values::tokens_of(x.s("limit")) as f64) { Ink::Peach } else { Ink::Soft };
        rows.push(("tokens", k.fact("Tokens", vec![Inked::of(tokens_line(x, counted), ink2)])));
    }
    let who = if under { String::new() } else if r.b("app") { if !c.agent && r.s("app") == crate::store::files::me() { "you".into() } else { r.t("app") } } else { String::new() };
    if !who.is_empty() { rows.push(("who", k.fact("On it", vec![Kit::quiet(clean(&who))]))); }
    // Every hand-over the task had, in plain words: beside the list the latest, full screen all of them.
    let overs = r.arr("handovers");
    if !overs.is_empty() {
        let shown: Vec<&Value> = if c.full { overs.iter().collect() } else { overs.iter().rev().take(1).collect() };
        let lines: Vec<Inked> = shown.iter().flat_map(|h| Kit::wrap_at(&format!("{} · {} ago", crate::store::handovers::handover_line(h), age(c.now - h.i("at"))), k.room)).map(|l| l.recolor(Ink::Soft)).collect();
        rows.push(("over", k.fact("Took over", lines)));
    }
    let made = crate::automations::told::made_by_words(r);
    if !made.is_empty() { rows.push(("made", k.fact("Made by", Kit::wrap_at(&made, k.room).into_iter().map(|l| l.recolor(Ink::Soft)).collect()))); }
    let st = crate::re!(r"(\d+) files? changed(?:, (\d+) insertions?\(\+\))?(?:, (\d+) deletions?\(-\))?");
    let size = match st.captures(r.s("stat")) { Some(m) => format!("{} file{}, +{} −{}", &m[1], if &m[1] == "1" { "" } else { "s" }, m.get(2).map(|x| x.as_str()).unwrap_or("0"), m.get(3).map(|x| x.as_str()).unwrap_or("0")), None => clean(r.s("stat")).trim().to_string() };
    let url = if crate::re!(r"^https?://\S+$").is_match(r.s("pr")) { r.t("pr") } else { String::new() };
    let pr = if r.b("pr") { let p = Inked::of(pr_number(r), pr_color(r)); let p = if url.is_empty() { p } else { p.link(&url) }; if merged(r) { p.add(Kit::dim(" merged")) } else { p } } else { Inked::new() };
    if !size.is_empty() || !pr.is_empty() {
        let mut v = k.flow(vec![pr, if size.is_empty() { Inked::new() } else { Kit::quiet(size) }], None, " · ");
        if c.full && !url.is_empty() && !c.web { v.push(Kit::dim(url.clone())); }
        rows.push(("change", k.fact("Change", v)));
    }
    if !c.agent && !r.b("archivedAt") && !r.at("pending").b("word") && r.s("state") == "idle" && r.s("description").trim().is_empty() { rows.push(("next", k.fact("Next", Kit::wrap_at(&format!("No agent is on this yet. {} puts one on it.", key_name("delegate")), k.room).into_iter().map(|l| l.recolor(Ink::Muted)).collect()))); }
    let top = Top { rows };
    let top_part = k.part(top.lines());
    k.parts[top_part].name = "top";
    // What it needs from you: a part of its own, its headline in the state's colour.
    let said = if c.agent { r.t("said") } else { String::new() };
    let mut told = String::new();
    if r.b("archivedAt") { k.alert(Ink::Muted, "Archived", &format!("Read-only. {} restores it. Its state and all its data are kept.", key_name("archive")), &[]); }
    if let Some(f) = failure.as_ref().filter(|_| waiting_of(r).map(|w| w.s("who") == "agent").unwrap_or(false)) {
        told = f.t("detail");
        let w = waiting_of(r).unwrap();
        k.alert(ink, "Cut off", &format!("Trying again by itself. {note}. A reply, {} or {} tells the agent to continue right away.", key_name("sendBack"), key_name("yes")), &[("Cut off by", if w.b("error") { w.t("error") } else { f.t("detail") })]);
    } else if let Some(f) = &failure {
        told = f.t("detail");
        let heading = if r.s("kind") == "auth" { asks("auth") } else { "Went wrong".into() };
        let next = if f.b("sentAt") { "Sent to the agent. Kept here until it works.".to_string() } else if f.b("agentCanFix") { format!("The agent can fix this: {} sends it.", key_name("sendBack")) } else { "This needs you first.".into() };
        let title = if same(f.s("title"), &heading) { String::new() } else { format!("{}\n", f.s("title")) };
        k.alert(Ink::Red, &heading, &format!("{title}{next}"), &[("Next step", if f.b("next") { f.t("next") } else { "Find the cause and fix it.".into() }), ("Details", f.t("detail"))]);
    } else if r.s("state") == "blocked" && r.s("stage") == "approval" && r.b("syncBlocked") {
        let said = r.at("syncBlocked").t("detail");
        k.alert(Ink::Peach, "Behind main", &format!("Cannot be approved yet: {}. The board keeps trying to bring it up to date.", r.at("syncBlocked").s("why")), &if said.is_empty() { vec![] } else { vec![("Details", said)] });
    } else if failed_review(r) {
        let tally = review_tally(r);
        k.alert(Ink::Red, "Review failed", &format!("{}{}", if tally.is_empty() { String::new() } else { format!("It found {tally}. ") }, if c.web { "Send the review to the agent to fix it, or merge it anyway: that asks first.".to_string() } else { format!("{} sends the review to the agent to fix. {} merges it anyway, and asks first.", key_name("sendBack"), key_name("yes")) }), &[]);
    } else if (r.s("state") == "blocked" && r.s("stage") == "approval") || r.s("stage") == "plan" {
    } else if stopped_at_limit(r) {
        let ls = r.at("limitStop");
        let review = ls.s("role") == "reviewer";
        told = note.clone();
        let on = if c.web { format!("{} with the buttons; a reply to the agent continues it with your words.", if review { "Carry on" } else { "Tell the agent to continue, or take its work as it is" }) }
            else { format!("{} carries on{}.", key_name("sendBack"), if review { if ls.b("ready") { if ls.b("done") { ": it was finished and reviewed, and is done".to_string() } else { ": its work was finished and reviewed, and is put up for approval".into() } } else if ls.b("review") { ": the review goes to the agent".into() } else { ": it is reviewed again".into() } } else { format!(", and so does a reply to the agent. {} takes the work as it is, through the check and the review", key_name("yes")) }) };
        let who = if review && !ls.b("review") && !ls.b("ready") { "The reviewer" } else { "It" };
        let raw = if r.b("raw") { r.t("raw") } else if !note.is_empty() { note.clone() } else { "stopped".into() };
        let id = if r.b("rid") { r.t("rid") } else { r.t("id") };
        k.alert(Ink::Red, "Stopped at a limit", &format!("{who} {}, {}. Its work is kept as it is and nothing was merged.\n{on}", crate::store::tasks::one_line(&raw, 600), used_say(ls)), &[("More", format!("A higher limit carries on by itself: herdr-projects set limit.{} <value> --task {id}", ls.s("what")))]);
    } else if r.s("stage") == "pick" { told = note.clone(); }
    else if r.s("state") == "blocked" { told = if !note.is_empty() { note.clone() } else { said.clone() }; let h = asks(r.s("kind")); let x = if same(&told, &h) { String::new() } else { told.clone() }; k.alert(Ink::Red, &h, &x, &[]); }
    else if r.s("state") == "error" {
        told = [if r.s("stage") == "nosummary" { r.t("screen") } else { String::new() }, note.clone(), said.clone()].into_iter().find(|x| !x.is_empty()).unwrap_or_else(|| "It failed".into());
        let h = if r.s("stage") == "nosummary" { "Stopped without a summary" } else { "Went wrong" };
        let x = if same(&told, h) { String::new() } else { told.clone() };
        k.alert(Ink::Red, h, &x, &[]);
    }
    (title, top, top_part, told)
}
