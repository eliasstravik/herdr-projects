//! The reviews in the task panel: how each went, each one under its own label, and the evidence. Beside the list only
//! the latest shows; full screen all of them, the newest first, each with what it found, what it checked and what it
//! could not verify. And the evidence a review brought: its links, each a name behind an arrow.
use super::kit::{Kit, Part};
use super::Ctx;
use crate::board::keys::key_name;
use crate::gate;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::shown::review_words::{quiet_review, verdict, verdict_mark};
use crate::shown::words::{age, brief, clean};

fn sev_ink(s: &str) -> Ink { match s { "critical" | "major" => Ink::Red, "minor" => Ink::Yellow, _ => Ink::Muted } }
/// Keep every distinct target; only exact duplicates are removed.
pub fn review_artifacts(list: &Value) -> Vec<Value> {
    let mut seen = vec![];
    list.as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter(|a| {
        if !a.is_object() { return false; }
        let key = [a.t("link"), a.t("url"), a.t("file")].into_iter().find(|k| !k.is_empty()).unwrap_or_default();
        if key.is_empty() || seen.contains(&key) { return false; }
        seen.push(key); true
    }).cloned().collect()
}
/// One piece of evidence: its name behind an arrow, wrapped whole, every line of it the link. `whole` (full screen)
/// adds what it is and the link printed in full.
pub fn artifact_rows(a: &Value, room: usize, whole: bool) -> Vec<Inked> {
    let squash = |v: &str| crate::text::squash(&crate::shown::words::clean(v));
    let lines = |t: &str| Inked::plain(squash(t)).wrap(room.saturating_sub(2).max(1));
    let url = [a.t("link"), a.t("url")].into_iter().find(|k| !k.is_empty()).unwrap_or_default();
    let linked = (url.starts_with("http://") || url.starts_with("https://")) && !url.chars().any(|c| (c as u32) <= 0x20 || c as u32 == 0x7f);
    let name = [a.t("caption"), a.t("file")].into_iter().find(|k| !k.is_empty()).unwrap_or_else(|| "Open evidence".into());
    let mut out: Vec<Inked> = lines(&name).into_iter().enumerate().map(|(n, l)| { let l = if linked { l.link(&url).under() } else { l }; if n > 0 { Inked::plain("  ").add(l) } else { Inked::of("↗ ", Ink::Accent).add(l) } }).collect();
    if whole { for l in if a.b("detail") { lines(a.s("detail")) } else { vec![] }.into_iter().chain(if url.is_empty() { vec![] } else { lines(&url) }) { out.push(Inked::plain("  ").add(l.recolor(Ink::Muted))); } }
    out
}
/// Returns whether something is shorter here than on the full screen.
pub fn reviews(k: &mut Kit, c: &Ctx, r: &Obj) -> bool {
    let hist = crate::store::reviews::review_history(r);
    let Some(last) = hist.last().cloned() else {
        if r.at("review").b("stale") { let mut p = Part::of(vec![k.head("Review", None), Kit::dim("The work changed since its review.")]); p.pri = 3; p.min = 2; p.at = Some(Box::new(|_| vec![])); k.parts.push(p); }
        return false;
    };
    let mut cut = false;
    let stale = r.at("review").b("stale");
    let name = |e: &Value| format!("Review {}", e.t("round"));
    let room = k.room;
    let result = |k: &Kit, e: &Value| -> Vec<Inked> {
        let q = quiet_review(r, e, *e == last);
        k.flow(vec![verdict_mark(e, q).add(Inked::plain(" ")).add(verdict(e, true, q).bold()), if e.b("at") { Kit::dim(format!("{} ago", age(c.now - e.i("at")))) } else { Inked::new() },
            if c.full && e.b("head") { Kit::dim(format!("at {}", &e.t("head").chars().take(7).collect::<String>())) } else { Inked::new() },
            if e.b("by") && crate::store::handovers::last_handover(r, "reviewer").is_some() { Kit::dim(format!("by {}", e.s("by"))) } else { Inked::new() }], None, " · ")
    };
    // A review that still stood after main was merged in says so, with what ran instead.
    let kept = |e: &Value| -> Vec<Inked> {
        let Some(kk) = e.arr("kept").last() else { return vec![] };
        let words = crate::store::reviews::kept_words(kk, e.i("round"));
        let mut ch = words.chars();
        let words = ch.next().map(|f| f.to_uppercase().collect::<String>() + ch.as_str()).unwrap_or_default();
        Kit::wrap_at(&format!("{words}{}{}", if kk.b("at") { format!(" · {} ago", age(c.now - kk.i("at"))) } else { String::new() }, if e.arr("kept").len() > 1 { format!(" · {} times", e.arr("kept").len()) } else { String::new() }), room).into_iter().map(|l| l.recolor(Ink::Muted)).collect()
    };
    let notes = |e: &Value| -> Vec<Inked> { let mut v: Vec<Inked> = if *e == last && stale && !e.b("error") { Kit::wrap_at("The work changed since this review.", room).into_iter().map(|l| l.recolor(Ink::Muted)).collect() } else { vec![] }; v.extend(kept(e)); v };
    let pad = if room >= 40 { 10 } else { 2 };
    let finding = |f: &Value, tail: &str| -> Vec<Inked> {
        let text = format!("{}{}{tail}", f.s("subject"), if f.b("where") { format!(" ({})", f.s("where")) } else { String::new() });
        Kit::wrap_at(&text, room.saturating_sub(pad)).into_iter().enumerate().flat_map(|(n, l)| if pad > 2 { vec![if n > 0 { Inked::spaces(pad).add(l) } else { Inked::of(crate::text::pad(f.s("severity"), pad), sev_ink(f.s("severity"))).add(l) }] } else { let mut v = vec![]; if n == 0 { v.push(Inked::of(f.t("severity"), sev_ink(f.s("severity")))); } v.push(Inked::plain("  ").add(l)); v }).collect()
    };
    let found = |k: &Kit, e: &Value| -> Vec<Inked> {
        let s = gate::split(e, None);
        let mut v: Vec<Inked> = s.required.iter().flat_map(|f| finding(f, "")).collect();
        v.extend(s.optional.iter().flat_map(|f| finding(f, " · not required")));
        v.extend(s.preexisting.iter().flat_map(|f| { let w = gate::went_to(f); finding(f, &format!(" · pre-existing{}", if w.is_empty() { String::new() } else { format!(", {w}") })) }));
        if e.i("left") > 0 { v.extend(Kit::wrap_at(&format!("{} smaller finding{} noticed late, left out", e.i("left"), if e.i("left") == 1 { "" } else { "s" }), room).into_iter().map(|l| l.recolor(Ink::Muted))); }
        k.fact("Findings", v)
    };
    let strs = |v: &Value| v.as_array().map(|a| a.iter().map(text).collect::<Vec<_>>()).unwrap_or_default();
    let whole = |k: &Kit, e: &Value| -> Vec<Inked> {
        let mut body = result(k, e); body.extend(notes(e)); body.extend(Kit::wrap_at(&[e.t("error"), e.t("text")].into_iter().find(|x| !x.is_empty()).unwrap_or_else(|| "No comment.".into()), room));
        let mut v = k.fact(&name(e), body);
        if gate::is_tagged(e) { if !e.arr("findings").is_empty() || e.i("left") > 0 { v.extend(found(k, e)); } } else { v.extend(k.items("Problems", &strs(e.at("problems")))); }
        v.extend(k.items("Left the plan", &strs(e.at("offPlan")))); v.extend(k.items("Checked", &strs(e.at("checks")))); v.extend(k.items("Unverified", &strs(e.at("limitations"))));
        v
    };
    let trail = |k: &Kit| k.fact("History", k.flow(hist.iter().map(|e| verdict(e, false, quiet_review(r, e, *e == last))).collect(), Some(room), " → "));
    let title = if hist.len() > 1 { format!("Reviews · {}", hist.len()) } else { "Review".into() };
    let evidence = review_artifacts(r.at("review").at("artifacts"));
    if c.full {
        let mut lines = vec![k.head(&title, None)];
        if hist.len() > 1 { lines.extend(trail(k)); lines.push(Inked::new()); }
        for (n, e) in hist.iter().rev().enumerate() { if n > 0 { lines.push(Inked::new()); } lines.extend(whole(k, e)); }
        k.part(lines);
        if !evidence.is_empty() { let mut l = vec![k.head("Evidence", None)]; for (n, a) in evidence.iter().enumerate() { if n > 0 { l.push(Inked::new()); } l.extend(artifact_rows(a, k.w, true)); } k.part(l); }
        return false;
    }
    let said = clean(&[last.t("error"), last.t("text")].into_iter().find(|x| !x.is_empty()).unwrap_or_else(|| "No comment.".into()));
    let all = Kit::wrap_at(&brief(&said, 280, " …"), room);
    let links: Vec<Inked> = if evidence.is_empty() { vec![] } else { let mut v = artifact_rows(&evidence[0], room, false); if evidence.len() > 1 { v.push(Kit::dim(format!("… {} more: {}", evidence.len() - 1, key_name("panel")))); } k.fact("Evidence", v) };
    let row = |k: &Kit, n: usize| { let mut v = result(k, &last); v.extend(notes(&last)); v.extend(Kit::first(&all, n, room)); k.fact(&name(&last), v) };
    // The full screen has more whenever the summary was shortened here or the review found or checked something.
    if hist.len() > 1 || brief(&said, 280, " …") != said || ["findings", "problems", "checks", "limitations", "offPlan"].iter().any(|x| !last.arr(x).is_empty()) || last.i("left") > 0 || last.b("head") || evidence.len() > 1 || evidence.iter().any(|a| a.b("detail") || a.b("link") || a.b("url")) { cut = true; }
    // Short of room the summary gives up lines, then the history goes, the evidence, the summary and the headline.
    let head = k.head(&title, None);
    let mut shapes: Vec<Vec<Inked>> = (0..all.len()).map(|n| { let mut v = vec![head.clone()]; if hist.len() > 1 { v.extend(trail(k)); } v.extend(row(k, all.len() - n)); v.extend(links.clone()); v }).collect();
    shapes.push([vec![head.clone()], row(k, 1), links.clone()].concat());
    shapes.push([vec![head.clone()], row(k, 1)].concat());
    shapes.push([vec![head.clone()], k.fact(&name(&last), result(k, &last))].concat());
    let q = quiet_review(r, &last, true);
    shapes.push(k.fact(&name(&last), vec![verdict_mark(&last, q).add(Inked::plain(" ")).add(verdict(&last, true, q).bold())]));
    let n = shapes.len();
    let mut p = Part::of(shapes[0].clone());
    p.pri = 3; p.min = shapes[n - 3].len(); p.last = Some(shapes[n - 1].len());
    p.at = Some(Box::new(move |m| shapes.iter().find(|s| s.len() <= m).cloned().unwrap_or_default()));
    p.name = "reviews";
    k.parts.push(p);
    cut
}
/// The report: that a task is research only (or a chat), and the link to the report its agent wrote.
pub fn report(k: &mut Kit, c: &Ctx, r: &Obj) {
    let chat = crate::chat_only::is_chat(r);
    let research = crate::research::is_research(r);
    let rep = r.at("report");
    let has = rep.is_object() && (rep.b("link") || rep.b("file"));
    if !research && !chat && !has { return; }
    let mut lines = vec![k.head(if chat { "Chat" } else if research { "Research only" } else { "Report" }, None)];
    if chat { lines.extend(k.wrap("No change to the repo · ends when you say.").into_iter().map(|l| l.recolor(Ink::Muted))); }
    else if research { lines.extend(k.wrap(if has { "Changes nothing in the repo." } else { "Changes nothing in the repo. No report yet." }).into_iter().map(|l| l.recolor(Ink::Muted))); }
    let room = if c.full { k.w } else { k.room + 2 };
    if has { lines.extend(artifact_rows(&json!({ "link": rep.at("link"), "caption": "Open report", "detail": if c.full { format!("{}{}", rep.s("file"), if rep.b("at") { format!(" · kept {} ago", age(c.now - rep.i("at"))) } else { String::new() }) } else { String::new() } }), room, c.full)); }
    // How to open it: the board's key, or the terminal's own click on a link (the web board's link is a plain one).
    if has && !c.web {
        let click = if rep.s("link").starts_with("http") { format!(", or {}+click", if crate::board::key_style::mac() { "cmd" } else { "ctrl" }) } else { String::new() };
        lines.extend(Kit::wrap_at(&format!("{} opens it{click}", key_name("openLink")), room.saturating_sub(2).max(1)).into_iter().map(|l| Inked::plain("  ").add(l.recolor(Ink::Muted))));
    }
    k.part(lines);
}
