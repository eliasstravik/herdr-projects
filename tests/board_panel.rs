//! The task panel beside the list: a long description cut short with where the rest is, every setting the task runs
//! with, what gives way first in a short window, the inks of headlines, labels and values; and the whole task full
//! screen, scrolled, with the place each of the task's own settings came from.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::json::*;
use sw::shown::ink::{Ink, Inked};
use sw::store::settings::change::{set_value, unset_value};
use sw::store::settings::lookup::Scope;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn keyed(re: &str) -> String { let mut out = re.to_string(); for k in ["panel", "settings", "sendBack", "yes"] { out = out.replace(&format!("{{{k}}}"), &regex::escape(&key_name(k))); } out }
fn add(w: &World, title: &str, description: &str) -> String { sw::actions::filing::add(&w.root, title, sw::actions::filing::AddOpts { description: description.into(), ..Default::default() }).unwrap().t("id") }
/// The selected task's panel as its lines' inks, at this width, whole or beside the list.
fn panel_of(b: &mut Board, width: usize, full: bool) -> Vec<Inked> { let r = b.s.current().unwrap().clone(); sw::board::view::panel::panel(&mut b.s, &b.theme, &r, width, full, 0) }
fn text_of(lines: &[Inked]) -> String { lines.iter().map(|l| l.text()).collect::<Vec<_>>().join("\n") }
/// The side panel's lines (what stands right of the bar) of a board this many rows high, on this task.
fn beside(w: &World, id: &str, rows: usize) -> Vec<String> { let mut b = open(&w.root, rows); b.s.sel = id.into(); b.load(); b.screen(120).iter().map(|l| l.split('│').nth(1).unwrap_or("").trim_end().to_string()).collect() }
const SETTINGS: [&str; 9] = ["Agent", "Plan first", "Reviewer", "Review when", "Send back", "Check", "Merge", "Workspace", "Also"];
fn every(side: &[String], at: &str) {
    for fact in SETTINGS { assert!(side.iter().any(|l| l.starts_with(&format!("  {fact} "))), "{at}: {fact}\n{}", side.join("\n")); }
    assert!(!side.iter().any(|l| has(l, "more settings?")), "{at}: no setting is left out");
}
fn set(name: &str, v: &str, id: &str) { set_value(name, &json!(v), &Scope::task(&task(id))).unwrap(); }

#[test]
fn the_panel_beside_the_list_cuts_a_long_description_short_and_the_whole_panel_shows_all_of_it() {
    let w = world();
    let long: Vec<String> = (1..=80).map(|i| format!("Line {i} of a very long description.")).collect();
    add(&w, "Short one", "Brief.");
    let big = add(&w, "Long one", &long.join("\n"));
    let mut s = open(&w.root, 44);
    s.s.sel = big.clone(); s.load();
    let text = s.screen(120).join("\n");
    assert!(has(&text, r"│ +{2}PRJ-2 Long one") && text.contains("Line 1 of a very long"), "{text}");
    // Beside the list the description is short: a few lines, then where the rest is.
    assert!(has(&text, r"Line 8 of a very long description\. … *\n") && has(&text, &keyed(r"│ +{2}{panel} shows the whole task")), "{text}");
    assert!(!text.contains("Line 9 of") && !text.contains("Line 80 of"));
    // The title, the status under its headline as facts, then the description, then the settings: an empty line between.
    assert!(has(&text, r"│ +{2}PRJ-2 Long one *\n[^\n]*│ +*\n[^\n]*│ +{2}Status ─+ *\n[^\n]*│ +{2}State {8}○ to do · \ds ago *\n[^\n]*│ +*\n[^\n]*│ +{2}Description ─+ *\n"), "{text}");
    assert!(has(&text, r"│ +{2}Settings ─+ *\n[^\n]*│ +{2}Agent {8}claude · default model *\n[^\n]*│ +{15}default effort *\n[^\n]*│ +{2}Plan first {3}off *\n"), "{text}");
    assert!(!text.contains("as usual"));
    let whole = text_of(&panel_of(&mut s, 80, true));
    // (A long folder, as macOS's temp folders are, goes on on a line of its own.)
    assert!(has(&whole, r"\n Settings ─+\n Project {6}\S+\n(?: {14}\S+\n)? Worktree {5}on\n Branch {7}main\n Agent "), "{whole}");
    // Every setting shows in both; where the task is (its project, its worktree, its branch) is on the whole task only.
    for fact in SETTINGS { assert!(whole.contains(&format!("\n {fact} ")), "{fact}"); assert!(text.contains(&format!("│  {fact} ")), "{fact}"); }
    for fact in ["Project", "Worktree", "Branch"] { assert!(whole.contains(&format!("\n {fact} ")), "{fact}"); assert!(!text.contains(&format!("│  {fact} ")), "{fact}"); }
    assert!(has(&whole, r"\n Review when {2}in a repo · with evidence\n Send back {4}major or worse · 3 rounds max\n Check {8}no command · 15m at most\n Merge {8}after your yes · squash · opens a pull request(\n {14}| · )waits for GitHub's checks\n Workspace {4}from main · removed when done\n Also {9}one attempt · not research only · no limits · no fallback\n"), "{whole}");
    assert!(!has(&whole, r"\((this task|you|this repo)\)|\*"), "no value names a place, and none is marked: none is this task's own");
    // A short window gives the settings its lines before the description: cut further, then the one line that says
    // where to read it, then gone, and only then a setting. With no room for all, a line counts the rest.
    { let side = beside(&w, &big, 32); every(&side, "32 rows");
      assert!(has(&side.join("\n"), r"\n {2}Description ─+\n {2}Line 1 of a very long description\.\n {2}Line 2 of a very long description\.\n {2}Line 3 of a very long description\. …\n\n {2}Settings ─"), "{}", side.join("\n")); }
    { let side = beside(&w, &big, 30); every(&side, "30 rows"); let t = side.join("\n");
      assert!(has(&t, &keyed(r"\n {2}State {8}○ to do · \ds ago\n\n {2}Description · {panel} shows it\n\n {2}Settings ─")) && !t.contains("Line 1 of"), "{t}"); }
    { let side = beside(&w, &big, 27); every(&side, "27 rows"); let t = side.join("\n"); assert!(!has(&t, "Description|Line 1 of"), "{t}"); }
    { let side = beside(&w, &big, 20); let t = side.join("\n");
      assert!(has(&t, &keyed(r"\n {2}Status ─+\n {2}State {8}○ to do · \ds ago\n\n {2}Settings ─+\n {2}Agent {8}claude · default model\n {15}default effort\n {2}Reviewer {5}codex · default model\n {15}default effort\n {2}Review when {2}in a repo · with evidence\n {2}Send back {4}major or worse · 3 rounds max\n {2}16 more settings · {settings} shows them\n\n {2}{panel} shows the whole task")), "{t}");
      assert!(!side.iter().any(|l| l.ends_with('…')), "nothing is cut in the middle of a line"); }
    // With too little room for all, a value somebody chose comes before the built-in ones.
    set("limit.time", "2h", &big);
    let t = beside(&w, &big, 20).join("\n");
    assert!(has(&t, r"\n {2}Settings ─+\n {2}Limits {7}2h a run \* · no token limit\n {2}Agent {8}claude · default model\n"), "{t}");
    assert!(!t.contains("this task only"), "squeezed, the line that says what * means goes first: {t}");
    unset_value("limit.time", &Scope::task(&task(&big))).unwrap();
    set("worker.agent", "claude/opus", &big); s.load();
    let text = s.screen(120).join("\n");
    assert!(has(&text, r"│ +{2}Agent {8}\S+ · opus \* *\n[^\n]*│ +{15}default effort *\n") && has(&text, r"│ +{2}Reviewer {5}codex · default model") && !text.contains("as usual"), "{text}");
    assert!(has(&text, r"│ +{2}\* this task only *\n"), "with room, one line says what * means: {text}");
    // A long text with no line breaks is cut between words after a few hundred characters.
    let prose = add(&w, "Prose", &vec!["word"; 400].join(" "));
    s.s.sel = prose; s.load();
    let side = s.screen(160).iter().map(|l| l.split('│').nth(1).unwrap_or("").to_string()).collect::<Vec<_>>().join(" ");
    let shown = regex::Regex::new(r"Description ─+\s+((?:word\s+)+)…").unwrap().captures(&side).map(|c| regex::Regex::new(r"\s+").unwrap().replace_all(&c[1], " ").len()).unwrap_or(0);
    assert!((280..=400).contains(&shown), "a description beside the list keeps to 280 to 400 characters: {shown}");
    s.s.sel = big.clone(); s.load();
    // A headline is bold and a label is greyed out. Every value is in the text colour, wherever it comes from; one
    // set for this task only has a `*` after it, in the accent colour.
    let lines = panel_of(&mut s, 58, false);
    let piece = |lines: &[Inked], f: &dyn Fn(&sw::shown::ink::Piece) -> bool| lines.iter().flat_map(|l| l.0.iter()).any(f);
    assert!(piece(&lines, &|p| p.t == "Description" && p.bold), "headlines are bold");
    let agent = lines.iter().find(|l| l.text().starts_with(" Agent ")).unwrap();
    assert_eq!(agent.0.iter().find(|p| p.t.starts_with("Agent")).map(|p| p.c), Some(Ink::Muted), "labels grey: {agent:?}");
    assert!(agent.0.iter().any(|p| p.t.contains("opus") && matches!(p.c, Ink::Text | Ink::Plain)), "a value chosen for this task is in the text colour (Plain is drawn in the text colour): {agent:?}");
    assert!(piece(&lines, &|p| p.t == "*" && p.c == Ink::Accent), "the * is in the accent colour");
    assert!(piece(&lines, &|p| p.t.contains("codex · default model") && matches!(p.c, Ink::Text | Ink::Plain)), "a built-in value looks as any other");
    assert!(lines.iter().find(|l| l.text().contains("Line 1 of a very long")).map(|l| l.0.iter().all(|p| p.c == Ink::Plain && !p.bold)).unwrap_or(false), "the text is neither");
    let whole = panel_of(&mut s, 100, true);
    let reviewer = whole.iter().find(|l| l.text().starts_with(" Reviewer ")).unwrap();
    assert!(reviewer.0.iter().any(|p| p.t.starts_with("codex") && matches!(p.c, Ink::Text | Ink::Plain)), "on the whole task a default setting looks as any other: {reviewer:?}");
    // The task details: the same parts full screen, scrolled, with every setting.
    s.press(&["alt+t"]);
    assert!(s.s.full);
    assert!(has(&s.screen(120).join("\n"), r"^ Task details {2}PRJ-2 \x22Long one\x22 +lines 1–\d+ of \d+"));
    s.s.panel_top = usize::MAX / 2; let text = s.screen(120).join("\n");
    assert!(text.contains("Line 80 of"));
    for fact in SETTINGS { assert!(text.contains(&format!("\n {fact} ")), "{fact}"); }
    // A choice made for this task alone has a `*` after its value, in both, and one line under them says what it means.
    for (k, v) in [("review.when", "always"), ("review.mustFix", "minor"), ("review.rounds", "5"), ("review.evidence", "false")] { set(k, v, &big); }
    s.load(); s.s.panel_top = usize::MAX / 2; let text = s.screen(120).join("\n");
    assert!(has(&text, r"\n Review when {2}always \* · no evidence \* *\n") && has(&text, r"\n Send back {4}minor or worse \* · 5 rounds max \* *\n"), "{text}");
    assert!(has(&text, r"opus \* · default effort *\n") && has(&text, r"\n \* this task only *\n"), "{text}");
    // A value from your own settings, or from the repo's file, is plain: the panel never says where it comes from.
    set_value("merge.method", &json!("rebase"), &Scope::default()).unwrap(); set_value("merge.by", &json!("itself"), &Scope::repo("proj")).unwrap();
    s.load(); s.s.panel_top = usize::MAX / 2;
    assert!(has(&s.screen(120).join("\n"), r"\n Merge {8}by itself, once it passes · rebase · opens a pull request( · |\s*│?\n[ │]*)waits for GitHub's checks *\n"), "{}", s.screen(120).join("\n"));
    unset_value("merge.method", &Scope::default()).unwrap(); unset_value("merge.by", &Scope::repo("proj")).unwrap(); s.load();
    s.press(&["up"]); assert!(s.s.panel_top < s.s.panel_length); assert_eq!(s.s.sel, big, "the arrows scroll the panel, they do not move the selection");
    s.press(&["esc"]); assert!(!s.s.full);
    s.s.sel = "proj/1".into(); s.load();
    let text = s.screen(120).join("\n");
    assert!(text.contains("Brief.") && !has(&text, "shows the whole task|…"), "a short one is left alone: {text}");
    assert!(!text.contains("None."), "and nothing is an empty box");
}

#[test]
fn the_panel_never_opens_full_screen_by_itself_something_going_wrong_only_changes_the_hint_line() {
    let w = world();
    let id = w.add("Match the colors");
    sw::store::tasks::update(&id, |t| { t.put("stage", "failed"); t.put("failure", json!({ "stage": "merge", "phase": "sync", "title": "Merge conflict with main", "detail": "Conflicting files:\nsrc/board/view.js", "agentCanFix": true, "next": "Resolve the conflict." })); Ok(sw::store::tasks::Change::Write) }).unwrap();
    let mut s = open(&w.root, 36);
    s.load();
    assert!(s.s.sel == id && !s.s.full && !s.s.reply);
    let text = s.screen(80).join("\n");
    assert!(has(&text, r"\n   ▾ needs you *\n[ ▌]  ✗ PRJ-1 Match the colors [^\n]*\n[ ▌] {10}Merge conflict with main"), "{text}");
    assert!(!text.contains("Conflicting files"));
    // The hint line: what matters now, the main thing first, never more than four; the main thing is the last to go.
    assert!(has(&s.screen(120).join("\n"), &keyed(r"\n[ ▌] {1}{sendBack} tell this task's agent to fix it · {yes} retry as it is · space talk about this task · \? all keys *$")), "{}", s.screen(120).join("\n"));
    assert!(has(&text, &keyed(r"\n[ ▌] {1}{sendBack} tell this task's agent to fix it · {yes} retry as it is · \? all keys *$")), "{text}");
    assert!(has(&s.screen(60).join("\n"), &keyed(r"\n[ ▌] {1}{sendBack} tell this task's agent to fix it · \? all keys *$")), "{}", s.screen(60).join("\n"));
    assert!(has(&s.screen(50).join("\n"), &keyed(r"\n[ ▌] {1}{sendBack} tell this task's agent to fix it *$")), "{}", s.screen(50).join("\n"));
    // Wide, the panel beside the list already says it, in order: where it stands, then what it needs from you.
    let text = s.screen(120).join("\n");
    assert!(has(&text, &keyed(r"│ +{2}State {8}✗ needs you · \ds ago *\n(?:[^\n]*│ +{2}On it[^\n]*\n)?[^\n]*│ +*\n[^\n]*│ +{2}Went wrong ─+ *\n[^\n]*│ +{2}Merge conflict with main *\n[^\n]*│ +{2}The agent can fix this: {sendBack} sends it\. *\n[^\n]*│ +{2}Next step {4}Resolve the conflict\. *\n[^\n]*│ +{2}Details {6}Conflicting files: *\n[^\n]*│ +{15}src/board/view\.js")), "{text}");
    // Narrow, space does what it does at every width: the chat names the task, and the list stays. alt+t opens the
    // same panel full screen; esc leaves it, and the chat stays the task's.
    s.cols = 80; s.screen(80); s.press(&["space"]);
    let text = s.screen(80).join("\n");
    assert!(!s.s.full && s.s.reply && text.starts_with(" herdr-projects") && has(&text, r"\n╭─ [^\n]* PRJ-1  · Agent ─+╮\n(?:│ +│\n)?"), "{text}");
    s.press(&["alt+t"]); let text = s.screen(80).join("\n");
    assert!(s.s.full && s.s.reply);
    assert!(text.starts_with(" Task details  PRJ-1 \"Match the colors\"") && text.contains("Merge conflict with main") && has(&text, r"\n Details {6}Conflicting files: *\n[ ▌] {13}src/board/view\.js"), "{text}");
    assert!(has(&text, r"\n╭─ [^\n]* PRJ-1  · Agent ─+╮\n(?:│ +│\n)?(?:[^\n]*\n)*?│ +❯"), "{text}"); // whole: the chat folds to its edge
    s.press(&["esc"]);
    let text = s.screen(80).join("\n");
    assert!(!s.s.full && s.s.reply && !text.contains("Conflicting files") && has(&text, r"\n╭─ [^\n]* PRJ-1  · Agent ─+╮\n(?:│ +│\n)?"), "{text}");
    // Nor does coming back from the archive bring it up, and an open panel is not remembered when the board reopens.
    s.press(&["alt+t"]); assert!(s.s.full);
    s.press(&["esc", "alt+a"]); assert!(s.s.archive_view);
    s.press(&["alt+a"]); assert!(!s.s.archive_view && s.s.sel == id && !s.s.full);
    s.press(&["alt+t"]); sw::board::run::prefs::save(&s.s);
    let mut again = open(&w.root, 34);
    again.load();
    assert!(again.s.sel == id && !again.s.full && !again.s.reply);
    assert!(!again.screen(80).join("\n").contains("Conflicting files"));
}
