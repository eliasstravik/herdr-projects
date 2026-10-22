//! The plan and the attempts in the task panel. The plan: where the plan step stands, who wrote it and who approves,
//! the plan's own words (its first lines beside the list) and its link. The attempts, for a task done several times
//! at once: the reviewer's comparison on top, then each attempt with its agent, where it stands, its time, tokens,
//! the size of its change, what the reviewer said of it and its link. The selected one is marked.
use super::kit::{Kit, Part};
use super::reviews::artifact_rows;
use super::Ctx;
use crate::attempts::words::{agent_word, mark_of, standing_of};
use crate::board::keys::key_name;
use crate::gate;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::shown::words::{age, tokens};

/// Beside the list the plan is its first lines; full screen it is whole.
const SHORT: usize = 12;
pub fn plan(k: &mut Kit, c: &Ctx, r: &Obj) -> bool {
    let p = r.at("plan");
    if !p.is_object() { return false; }
    let w = p.at("writer");
    let who = if w.is_object() { format!("{}{}", w.s("kind"), if p.b("model") || w.b("model") { format!(" · {}", if p.b("model") { p.s("model") } else { w.s("model") }) } else { String::new() }) } else { "the working agent".into() };
    let yes = if p.b("approvedAt") { format!("approved by {} {} ago", match p.s("approvedBy") { "reviewer" => "the reviewer", "nobody" => "nobody: plan.approve was taken off", _ => "you" }, age(c.now - p.i("approvedAt"))) }
        else if r.s("stage") == "plan" { "waits for your yes".into() } else if p.b("at") { "written".into() } else { "being written".into() };
    let mut lines = vec![k.head("Plan", None)];
    lines.extend(k.fact_text("Stands", &yes));
    lines.extend(k.fact("Written by", Kit::wrap_at(&who, k.room).into_iter().map(|l| l.recolor(Ink::Muted)).collect()));
    // A planner's time and tokens count to the task: said here, since the session below is the working agent's.
    if p.b("took") { let u = p.at("usage"); let x = [age(p.i("took")), if u.is_object() { format!("{} in · {} out", tokens(u.f("in")), tokens(u.f("out"))) } else { String::new() }].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · "); lines.extend(k.fact("Planner used", Kit::wrap_at(&x, k.room).into_iter().map(|l| l.recolor(Ink::Muted)).collect())); }
    let rv = p.at("review");
    if rv.is_object() {
        lines.extend(k.fact_text("Reviewer", &format!("{}{}", gate::result(rv, "merge"), if rv.b("text") { format!(". {}", rv.s("text")) } else { String::new() })));
        if !p.b("approvedAt") || c.full { for f in rv.arr("findings") { lines.extend(k.wrap(&format!("- {}", gate::finding_line(f)))); } }
    }
    if p.at("undone").is_object() { let u = p.at("undone"); let n = u.i("n"); lines.extend(k.fact_text("Put back", &format!("{n} {} made in the repo before the yes{}", if n == 1 { "change" } else { "changes" }, if c.full { format!(": {}", strings(u.at("list")).join(", ")) } else { String::new() }))); }
    if p.b("backs") { lines.extend(k.fact("Sent back", vec![Kit::dim(format!("{} {}", p.i("backs"), if p.i("backs") == 1 { "time" } else { "times" }))])); }
    let mut cut = false;
    if p.b("at") {
        // The plan's own words, from the task's file; another machine's task has its link only.
        let text = if r.b("host") { String::new() } else { r.at("snap").t("plan") };
        let all = if text.is_empty() { vec![] } else { k.wrap(&text) };
        let shown: Vec<Inked> = if c.full { all.clone() } else { all.iter().take(SHORT).cloned().collect() };
        if !shown.is_empty() { lines.push(Inked::new()); lines.extend(shown.iter().map(|l| l.clone().role("text"))); }
        if shown.len() < all.len() { cut = true; lines.push(Kit::dim(format!("… {} more lines", all.len() - shown.len()))); }
        lines.extend(artifact_rows(&json!({ "link": p.at("link"), "caption": "Open plan", "detail": if c.full { format!("{} · written {} ago", if p.b("file") { p.s("file") } else { "plan.md" }, age(c.now - p.i("at"))) } else { String::new() } }), if c.full { k.w } else { k.room + 2 }, c.full));
    }
    if r.s("stage") == "plan" && !r.b("archivedAt") {
        lines.push(Inked::new());
        let said = if c.web { "Yes approves this plan and the build starts. A reply sends it back with your words, and it is written again.".to_string() } else { format!("{} approves this plan and the build starts. A reply ({}) sends it back with your words, and it is written again.", key_name("yes"), key_name("talk")) };
        lines.extend(k.wrap(&said).into_iter().map(|l| l.recolor(Ink::Muted)));
    }
    k.part(lines);
    cut
}
/// "3 files changed, 10 insertions(+), 2 deletions(-)" -> "3 files · +10 −2".
pub fn size_of(stat: &str) -> String {
    let n = |re: &str| regex::Regex::new(re).unwrap().captures(stat).and_then(|m| m[1].parse::<i64>().ok()).unwrap_or(0);
    let files = n(r"(\d+) files? changed");
    if files == 0 { return String::new(); }
    format!("{files} file{} · +{} −{}", if files == 1 { "" } else { "s" }, n(r"(\d+) insertions?"), n(r"(\d+) deletions?"))
}
pub fn attempts(k: &mut Kit, c: &Ctx, r: &Obj) -> bool {
    let tries: Vec<Obj> = r.arr("tries").iter().filter_map(|x| x.as_object().cloned()).collect();
    if tries.is_empty() { return false; }
    let passed = tries.iter().filter(|x| standing_of(x, c.now) == "passed").count();
    let cmp = r.at("comparison");
    let sel = c.attempt.clone();
    let ink = crate::shown::states::status_ink(r);
    let mut lines = vec![if r.s("stage") == "pick" { k.head(&format!("Pick one of {passed}"), Some(ink)) } else { k.head("Attempts", None) }];
    let by = |b: &str| match b { "you" => "by you".to_string(), "reviewer" => "by the reviewer".into(), "check" => "the only one that passed".into(), x => x.into() };
    if r.at("picked").is_object() { lines.extend(k.fact("Chosen", vec![Inked::plain(format!("{} · {}", r.at("picked").s("letter"), by(r.at("picked").s("by"))))])); }
    if cmp.b("pick") { lines.extend(k.fact_text("Reviewer", &format!("recommends {}{}", cmp.s("pick"), if cmp.b("why") { format!(": {}", cmp.s("why")) } else { String::new() }))); }
    else if cmp.b("error") { lines.extend(k.fact("Reviewer", Kit::wrap_at(&format!("no comparison: {}", cmp.s("error")), k.room).into_iter().map(|l| l.recolor(Ink::Yellow)).collect())); }
    else if r.s("stage") == "reviewing" && r.b("comparing") { lines.extend(k.fact("Reviewer", vec![Kit::dim("is comparing them")])); }
    // Beside the list, once the choice is made, each attempt is one fact.
    let short = !c.full && r.s("stage") != "pick";
    for x in &tries {
        let st = standing_of(x, c.now);
        let spec = r.arr("attempts").iter().find(|a| a.s("letter") == x.s("letter")).map(|a| a.t("agent")).unwrap_or_else(|| agent_word(x));
        let full_mark = mark_of(x, c.now);
        let word = full_mark.get(format!("{} {} ", x.s("letter"), agent_word(x)).len()..).unwrap_or("").to_string();
        let where_ = match st { "passed" => Inked::of(if x.at("checked") == &json!(false) { "✓ finished" } else { "✓ passed the check" }, Ink::Green), "out" | "needs" => Inked::of(word, Ink::Red), _ => Inked::of(word, Ink::Yellow) };
        let used = if x.at("usage").is_object() { format!("{} tokens", tokens(x.at("usage").f("in") + x.at("usage").f("out"))) } else { String::new() };
        let end = if ["passed", "out"].contains(&st) { if x.b("triedAt") { x.i("triedAt") } else if x.b("at") { x.i("at") } else { c.now } } else { c.now };
        let time = age(end - if x.b("since") { x.i("since") } else if x.b("created") { x.i("created") } else { c.now });
        let mut facts = k.flow(vec![Inked::plain(spec).bold(), where_, Inked::plain(time), Inked::plain(used), Inked::plain(size_of(x.s("stat")))], Some(k.room), " · ");
        if cmp.at("notes").b(x.s("letter")) && !short { facts.extend(Kit::wrap_at(cmp.at("notes").s(x.s("letter")), k.room).into_iter().map(|l| l.recolor(Ink::Muted))); }
        if st == "out" && x.at("failure").b("title") && c.full { facts.extend(Kit::wrap_at(x.at("failure").s("title"), k.room).into_iter().map(|l| l.recolor(Ink::Muted))); }
        let link = if x.at("report").b("link") { Some(json!({ "link": x.at("report").at("link"), "caption": "Open its report" })) } else if x.at("diff").b("link") { Some(json!({ "link": x.at("diff").at("link"), "caption": "Open its diff" })) } else { None };
        if let (Some(mut l), false) = (link.clone(), short) { if c.full { l.as_object_mut().unwrap().put("detail", crate::store::marks::tilde(x.s("worktree"))); } facts.extend(artifact_rows(&l, k.room, c.full)); }
        else if c.full && x.b("worktree") { facts.push(Kit::dim(crate::store::marks::tilde(x.s("worktree")))); }
        let mark = if x.s("letter") == sel { " ›" } else if r.at("picked").s("letter") == x.s("letter") || (!r.b("picked") && cmp.s("pick") == x.s("letter")) { " ★" } else { "" };
        let rows = k.fact(&format!("{}{mark}", x.s("letter")), facts);
        lines.extend(rows.into_iter().enumerate().map(|(n, l)| if n == 0 && x.s("letter") == sel { l.bold() } else { l }));
    }
    if r.s("stage") == "pick" { lines.extend(k.wrap(&format!("{} moves to the next attempt, {} keeps the one marked ›, {} goes into its tab.", key_name("nextAttempt"), key_name("yes"), key_name("open"))).into_iter().map(|l| l.recolor(Ink::Muted))); }
    // Short of room it gives up lines from its end, like every long part: while it waits for you, last of all.
    let all = lines.clone();
    let w = k.w;
    let mut p = Part::of(lines);
    if r.s("stage") == "pick" { p.alert = Some(Box::new(move |n| Kit::first(&all, n, w))); } else { p.pri = 3; p.min = 3; p.at = Some(Box::new(move |n| if n < 3 { vec![] } else { Kit::first(&all, n, w) })); }
    k.parts.push(p);
    short && (cmp.b("pick") || tries.iter().any(|x| x.at("diff").b("link") || x.at("report").b("link")))
}
