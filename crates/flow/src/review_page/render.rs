//! The evidence page itself: the work, the reviewer's findings as the board's gate reads them, and only the evidence
//! the reviewer selected. One portable HTML file; its look is page.css with the board's colours (board/theme.rs).
use crate::gate;
use crate::json::*;
use std::path::Path;

pub const REVIEW_FILE: &str = "review.html";
pub fn clean(v: &Value) -> String { text(v).chars().map(|c| if (c as u32) < 0x20 || c as u32 == 0x7f { ' ' } else { c }).collect::<String>().trim().to_string() }
pub fn esc(v: &str) -> String { v.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;") }
/// A link that may be shown: http or https, no user or password, nothing hidden in it.
pub fn web_url(raw: &str) -> String {
    if raw.chars().any(|c| (c as u32) <= 0x20 || c as u32 == 0x7f) { return String::new(); }
    let lower = raw.to_lowercase();
    let Some(rest) = lower.strip_prefix("https://").or_else(|| lower.strip_prefix("http://")) else { return String::new() };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() || host.contains('@') { return String::new(); }
    if rest.contains('/') || rest.contains('?') || rest.contains('#') { raw.to_string() } else { format!("{raw}/") }
}
/// No directory sweep: scratch files, duplicate captures and unlisted logs stay out of the presentation.
pub fn select_evidence(list: &Value, dir: &Path) -> Vec<Obj> {
    let mut out: Vec<Obj> = vec![];
    let mut seen = std::collections::HashSet::new();
    for a in list.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
        if !a.is_object() { continue; }
        let caption: String = clean(a.at("caption")).chars().take(160).collect();
        let detail: String = clean(a.at("detail")).chars().take(600).collect();
        let url = web_url(a.s("url"));
        let item = if !url.is_empty() { Some(obj(json!({ "url": url, "caption": if caption.is_empty() { "Try the work".into() } else { caption.clone() }, "detail": detail }))) }
        else if let Some(f) = a.at("file").as_str() {
            let plain = !f.is_empty() && !f.contains('/') && !f.contains('\\') && !f.chars().any(|c| (c as u32) < 0x20 || c as u32 == 0x7f) && !f.starts_with('.') && f != REVIEW_FILE;
            let is_file = std::fs::symlink_metadata(dir.join(f)).map(|m| m.is_file()).unwrap_or(false);
            (plain && is_file).then(|| obj(json!({ "file": f, "caption": if caption.is_empty() { f.to_string() } else { caption.clone() }, "detail": detail })))
        } else { None };
        let Some(item) = item else { continue };
        let key = if item.b("url") { item.t("url") } else { item.t("file") };
        if seen.insert(key) { out.push(item); }
    }
    out
}
fn list_html(items: &[Value]) -> String { format!("<ul>{}</ul>", items.iter().map(|s| format!("<li>{}</li>", esc(&text(s)))).collect::<String>()) }
fn paragraphs(v: &str) -> String {
    let re = crate::re!(r"\n\s*\n");
    re.split(v.trim()).filter(|s| !s.is_empty()).map(|s| format!("<p class=\"preserve\">{}</p>", esc(s))).collect()
}
fn ext_of(f: &str) -> String { Path::new(f).extension().map(|e| format!(".{}", e.to_string_lossy().to_lowercase())).unwrap_or_default() }
fn evidence_html(a: &Obj, i: usize, dir: &Path) -> String {
    let is_url = a.b("url");
    let href = if is_url { a.t("url") } else { format!("./{}", crate::util::url_encode(a.s("file"))) };
    let ext = ext_of(a.s("file"));
    let (cap, h) = (esc(a.s("caption")), esc(&href));
    let (kind, media) = match ext.as_str() {
        ".png" | ".jpg" | ".jpeg" | ".gif" | ".webp" => ("Screenshot", format!("<a class=\"capture\" href=\"{h}\"><img src=\"{h}\" alt=\"{cap}\" loading=\"lazy\"></a>")),
        ".mp4" | ".webm" => ("Recording", format!("<video controls preload=\"metadata\" aria-label=\"{cap}\"><source src=\"{h}\">Your browser cannot play this recording. Use the link below.</video>")),
        ".txt" | ".log" | ".md" | ".json" => {
            // A bounded, escaped peek; the complete selected file is still one click away.
            let mut buf = vec![0u8; 6001];
            let n = std::fs::File::open(dir.join(a.s("file"))).and_then(|mut f| { use std::io::Read; f.read(&mut buf) }).unwrap_or(0);
            let preview = format!("{}{}", String::from_utf8_lossy(&buf[..n.min(6000)]), if n > 6000 { "\n… Open the full file for the rest." } else { "" });
            ("Test / command output", format!("<details><summary>Read output</summary><pre tabindex=\"0\" role=\"region\" aria-label=\"{cap} output\">{}</pre></details>", esc(&preview)))
        }
        _ => (if is_url { "Try it" } else { "Evidence" }, String::new()),
    };
    let open = if is_url { "Open preview" } else { "Open full file" };
    format!("<article id=\"evidence-{n}\" class=\"evidence\"><div class=\"eyebrow\">{n:02} · {kind}</div><h3>{cap}</h3>{}{media}<p class=\"file-link\"><a href=\"{h}\" aria-label=\"{}\"{}>{open} <span aria-hidden=\"true\">↗</span></a></p></article>",
        if a.b("detail") { format!("<p>{}</p>", esc(a.s("detail"))) } else { String::new() }, esc(&format!("{open}: {}", a.s("caption"))), if is_url { " target=\"_blank\" rel=\"noopener noreferrer\"" } else { "" }, n = i + 1)
}
fn finding_html(f: &Value, note: &str) -> String {
    format!("<li><span class=\"sev {s}\">{s}</span> {}{}{}{}</li>", esc(f.s("subject")),
        if f.b("where") { format!(" <code>{}</code>", esc(f.s("where"))) } else { String::new() },
        if note.is_empty() { String::new() } else { format!(" <span class=\"tag\">{}</span>", esc(note)) },
        if f.b("proof") { format!("<div class=\"proof\">Proof: {}</div>", esc(f.s("proof"))) } else { String::new() }, s = esc(f.s("severity")))
}
fn group(title: &str, items: &[Value], note: &dyn Fn(&Value) -> String) -> String {
    if items.is_empty() { return String::new(); }
    format!("<h4>{title}</h4><ul class=\"findings\">{}</ul>", items.iter().map(|f| finding_html(f, &note(f))).collect::<String>())
}
/// `ctx`: id, round, title, description, summary, stat, head, pr.
pub fn render_review_page(ctx: &Value, review: &Value, evidence: &[Obj], dir: &Path) -> String {
    let found = gate::split(review, None);
    let tone = if review.b("pass") { "success" } else { "warning" };
    let left = review.i("left");
    let findings = group("Required", &found.required, &|_| String::new()) + &group("Not required: below the gate", &found.optional, &|_| "not required".into())
        + &group("Not required: there before this change", &found.preexisting, &|f| { let w = gate::went_to(f); if w.is_empty() { "not required · a note".into() } else { format!("not required · {w}") } })
        + &if left > 0 { format!("<p class=\"hint\">{left} smaller finding{} noticed only in this later round {} left out.</p>", if left == 1 { "" } else { "s" }, if left == 1 { "was" } else { "were" }) } else { String::new() };
    let checks = review.arr("checks");
    let limits = review.arr("limitations");
    let when = crate::clock::iso(review.i("at"));
    let when_shown = crate::re!(r"\.\d+Z$").replace(&when.replacen('T', " · ", 1), " UTC").to_string();
    let (id, title, round) = (ctx.t("id"), ctx.t("title"), ctx.t("round"));
    let name = esc(if title.is_empty() { &id } else { &title });
    let round_part = if round.is_empty() || round == "0" { String::new() } else { format!(" {}", esc(&round)) };
    let pr = web_url(ctx.s("pr"));
    let summary = paragraphs(ctx.s("summary"));
    let rtext = paragraphs(review.s("text"));
    let head = ctx.t("head");
    let tally = gate::tally(review);
    format!(r##"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta name="referrer" content="no-referrer"><title>{name} — Review</title>
<style>
{theme}{css}</style></head><body><a class="skip" href="#overview">Skip to review content</a><header class="top"><div class="top-inner"><div class="brand">herdr-projects <span class="hint">/ Review</span></div><nav aria-label="Review sections"><a href="#overview">Work</a><a href="#review">Review</a><a href="#evidence">Evidence</a></nav></div></header>
<main><div class="eyebrow">Review{round_part} · {eid}</div><h1>{name}</h1><div class="meta"><span><time datetime="{when}">{when_shown}</time></span>{commit}{prlink}</div>
<div class="layout"><div class="content"><section id="overview"><h2>What changed</h2>{summary}{stat}{desc}</section>
<section id="review"><h2>The review</h2>{rtext}<h3>Findings</h3>{findings}<h3>What was checked</h3>{checks}<h3>Limits &amp; unverified areas</h3>{limits}</section>
<section id="evidence"><h2>Selected evidence</h2><p class="hint">Only material chosen to help judge this work. Each item explains a result or gives you something to try; this is not a dump of every output.</p><ul class="evidence-nav" role="list">{nav}</ul><div class="evidence-list">{items}</div></section></div>
<aside class="verdict" aria-label="Review verdict"><div class="eyebrow">Reviewer verdict{verdict_round}</div><div class="score {tone}">{word}</div><p class="decision">{tally}</p><p>{question} <strong>{meets}</strong></p><p class="hint">The reviewer tags what it found; the board decides. Gate: a {gatew} finding{worse} must be fixed. This is not an approval.</p><a href="#review">Read findings ↓</a><br><a href="#evidence">See evidence ↓</a></aside></div><footer>Review of the work at the time shown above. Preview links may stop working when their servers stop. Approve or request changes from the work board.</footer></main></body></html>"##,
        theme = crate::board::theme::page_theme(), css = include_str!("page.css"), eid = esc(&id),
        commit = if head.is_empty() { String::new() } else { format!("<span>Commit <code>{}</code></span>", esc(&head.chars().take(12).collect::<String>())) },
        prlink = if pr.is_empty() { String::new() } else { format!("<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{} request ↗</a>", esc(&pr), if crate::flow::gitlab_words::is_mr(&pr) { "Merge" } else { "Pull" }) },
        summary = if summary.is_empty() { "<p class=\"empty\">No work summary was supplied.</p>".into() } else { summary },
        stat = if ctx.b("stat") { format!("<details><summary>Change size</summary><pre tabindex=\"0\" role=\"region\" aria-label=\"Change size\">{}</pre></details>", esc(&ctx.t("stat"))) } else { String::new() },
        desc = if ctx.b("description") { format!("<details><summary>Original request</summary>{}</details>", paragraphs(ctx.s("description"))) } else { String::new() },
        rtext = if rtext.is_empty() { "<p class=\"empty\">No review summary was supplied.</p>".into() } else { rtext },
        findings = if findings.is_empty() { "<p class=\"empty\">No findings reported by the reviewer.</p>".into() } else { findings },
        checks = if checks.is_empty() { "<p class=\"empty\">The reviewer did not list individual checks. See the selected evidence below.</p>".into() } else { list_html(checks) },
        limits = if limits.is_empty() { "<p class=\"empty\">No limits were listed. This is not a claim of exhaustive coverage.</p>".into() } else { list_html(limits) },
        nav = evidence.iter().enumerate().map(|(i, a)| format!("<li><a href=\"#evidence-{n}\">{n:02} · {}</a></li>", esc(a.s("caption")), n = i + 1)).collect::<String>(),
        items = evidence.iter().enumerate().map(|(i, a)| evidence_html(a, i, dir)).collect::<String>(),
        verdict_round = if round_part.is_empty() { String::new() } else { format!(" · review{round_part}") },
        word = gate::word(review), tally = esc(if tally.is_empty() { "Nothing found" } else { &tally }), question = esc(gate::QUESTION),
        meets = if review.b("meets") { "Yes" } else { "No" }, gatew = esc(review.s("gate")), worse = if review.s("gate") == "critical" { "" } else { " or worse" })
}
