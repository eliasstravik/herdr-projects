//! The web page's own code and styles, read as text (no browser runs here): a row's tag in a column of its own that
//! gives way on a narrow list, an automation's row and mark, the chat line with only what there is and the box's frame
//! colour by who gets the words, a draft kept with who it is for, and the styles a thumb and the eye need.
use regex::Regex;

const LIST: &str = include_str!("../crates/web/src/webui/page/list.js");
const CHAT: &str = include_str!("../crates/web/src/webui/page/chat.js");
const DOM: &str = include_str!("../crates/web/src/webui/page/dom.js");
const SHEETS: &str = include_str!("../crates/web/src/webui/page/sheets.js");
const CSS: &str = include_str!("../crates/web/src/webui/page/board.css");
const COMPOSE_CSS: &str = include_str!("../crates/web/src/webui/page/compose.css");
fn has(text: &str, re: &str) -> bool { Regex::new(re).unwrap().is_match(text) }

#[test]
fn a_rows_tag_has_a_column_of_its_own_and_a_narrow_list_drops_every_tag_at_once() {
    // the tag beside the lines, never inside the first one, as wide as the board's column says
    assert!(LIST.contains(r#"r.tag ? h("span.sw-id", { style: `min-width:${r.tagWidth || r.tag.length}ch` }, r.tag) : null,"#));
    assert!(has(LIST, r#"h\("span\.sw-main",\s*(//[^\n]*\n\s*)*r\.tag \? h\("span\.sw-id""#), "the tag comes first in the row's main part");
    assert!(LIST.contains(r#"h("span.sw-line", h(`span.sw-title${r.strong ? ".sw-b" : ""}`, titleOf(r)), h("span.sw-time", r.time))"#));
    // an automation's row is marked as one, so the styles show all of what it says; a task's row is not
    assert!(LIST.contains(r#"${r.automation ? " is-automation" : ""}"#));
    // what follows a title ([auto], [research], ×3) is dimmed, never in a colour; an automation's mark is an arrow going
    // round, drawn at the round marks' size, the same one quiet while it is off
    assert!(LIST.contains(r#"h("span.sw-count", t.slice(at))"#) && !LIST.contains(r#"h("span.sw-c-auto", "[auto]")"#));
    // the pin mark last is a yellow star, apart from the dimmed tags
    assert!(LIST.contains(r#"const PIN = " ★";"#) && LIST.contains(r#"h("span.sw-pin", PIN.trim())"#) && has(CSS, r"\.sw-pin \{ color: var\(--sw-yellow\); \}"));
    assert!(DOM.contains(r#"mark === TURN ? h("span.sw-turn")"#) && DOM.contains(r#"${dim ? ".sw-d" : ""}"#) && DOM.contains(r#"span.sw-mark${color ? `.sw-c-${color}` : ""}"#));
    for part in [r"\.sw-main \{ display: grid; grid-template-columns: auto minmax\(0, 1fr\)", r"\.sw-main > \* \{ grid-column: 2; \}", r"\.sw-no-tags \.sw-main > \.sw-id \{ display: none; \}",
        r"\.sw-word \{ display: none; \} \.sw-no-tags \.sw-word \{ display: inline; \}", r"\.sw-c-auto \{ color: var\(--sw-auto\); \}", r"--sw-auto: color-mix\(in srgb, #a855f7 78%"] { assert!(has(CSS, part), "{part}"); }
    // a narrow list: an automation's word goes with the tags (every title at one left edge) and is the first word of what its row says
    assert!(LIST.contains(r#"r.automation && r.tag ? h("span.sw-word", `${r.tag} · `) : null"#));
    // The tags give way together where the widest would leave a title fewer than 16 letters; a wider tag goes sooner.
    let fit = |px: f64, widest: f64, time: f64| -> bool { widest == 0.0 || px <= 0.0 || px >= 24.0 + 30.0 + widest * 7.3 + 8.0 + 16.0 * 7.5 + if time > 0.0 { 12.0 + time * 6.6 } else { 0.0 } };
    assert!(LIST.contains("const NAME_LEFT = 16, PAD = 24, MARK_W = 30, TAG_CH = 7.3, TAG_GAP = 8, TITLE_CH = 7.5, TIME_CH = 6.6, TIME_GAP = 12;"), "the numbers checked below");
    assert!(LIST.contains("export const tagsFit = (px, widest, time = 0) => !widest || px <= 0 || px >= PAD + MARK_W + widest * TAG_CH + TAG_GAP + NAME_LEFT * TITLE_CH + (time ? TIME_GAP + time * TIME_CH : 0);"));
    assert_eq!([fit(400.0, 4.0, 6.0), fit(200.0, 4.0, 6.0), fit(270.0, 4.0, 6.0), fit(270.0, 9.0, 6.0), fit(100.0, 0.0, 0.0), fit(0.0, 4.0, 0.0)], [true, false, true, false, true, true]);
    assert!(LIST.contains(r#"new ResizeObserver(() => el.classList.toggle("sw-no-tags", !tagsFit(el.clientWidth, widest, time))).observe(el);"#));
}

#[test]
fn the_chat_line_shows_only_what_there_is_and_the_boxs_frame_says_who_gets_the_words() {
    // Only what there is: replaceChildren would write a missing part out as the word "null".
    assert!(CHAT.contains("const mark = markEl(c.mark.t, c.mark.c);") && CHAT.contains(r#"const words = cap([c.status, c.progress].filter(Boolean).join(" · ")), kind = c.title?.kind || "";"#));
    assert!(CHAT.contains("[choice(c.top, false, null), choice(lit, true, mark)] : [choice(lit, true, mark), choice(c.row, false, rowMark)]"));
    // The box's frame: the plain text colour for the Coordinator, the task's status colour for its agent.
    assert!(CHAT.contains("tone = c.box || \"text\";") && CHAT.contains("card.className = `sw-card sw-b-${to && (moved || !now) ? \"text\" : tone}`;"));
    for c in ["text", "red", "green", "yellow", "teal", "review", "check", "todo", "overlay0"] { assert!(COMPOSE_CSS.contains(&format!(".sw-card.sw-b-{c}")), "the page has a frame colour for {c}"); }
    assert!(has(CSS, r"\.sw-c-todo \{ color: var\(--sw-todo\)"));
    let var = |n: &str| Regex::new(&format!(r"--sw-{n}: ([^;]+);")).unwrap().captures(CSS).map(|c| c[1].to_string()).unwrap_or_else(|| panic!("no --sw-{n}"));
    let all: std::collections::BTreeSet<String> = ["text", "red", "green", "yellow", "teal", "review", "todo"].iter().map(|n| var(n)).collect();
    assert_eq!(all.len(), 7, "each frame colour is a colour of its own: done is not the green of ready");
    // A reload: the saved words come back with who they were for, before the page knows who it shows; while another
    // task shows, the chip says so and the frame is the plain colour.
    assert!(CHAT.contains("chip.textContent = moved ? `To: ${to.label} · not who is shown above` : \"\";"));
    assert!(CHAT.contains("label: lit ? `${possessive(lit)} ${c.title.kind.toLowerCase()}` : c.name"), "named as the terminal names an agent: HP-12's agent");
}

#[test]
fn the_pages_styles_nothing_under_13px_targets_a_thumb_can_hit_and_a_draft_kept_with_who_it_is_for() {
    assert!(CSS.contains("--fs-2xs: 13px; --fs-xs: 13px;")); assert!(!CSS.contains("text-transform: uppercase"));
    assert!(!has(CSS, r"min-height: (2\d|3\d)px"), "no tap target under 40px");
    for part in ["full", "right", "clock"] { assert!(CSS.contains(&format!(".sw-dot-{part} {{")), "{part}"); }
    assert!(CSS.contains(".sw-board.has-task .sw-back { display: none; }"), "one back arrow at a time on a phone");
    // the chat full screen hides the header with the open task and their arrows: the full screen button, in its place,
    // goes back then (chat.js), so there is always a way back
    let full = include_str!("../crates/web/src/webui/page/chat.css");
    assert!(full.contains(".sw-board:is(.sw-chat-full, .sw-web-typing) > :is(.sw-head, .sw-shows, .sw-body,") && !full.contains("sw-chat-back"));
    // the chat's switch is a thumb's size to tap, though drawn smaller (the web UI's own expander)
    assert!(full.contains("@media (pointer: coarse) { .sw-chatbar .icon-button { min-width: var(--touch-target, 40px); min-height: var(--touch-target, 40px); } .sw-who-pick::before { content: \"\"; position: absolute; inset: -4px 0; } }"));
    assert!(has(CSS, r"\.sw-row\.is-selected \{[^}]*box-shadow: inset 3px 0 0 var\(--accent"), "the selected row has a bar in its gutter, not a tint alone");
    // The draft is kept with who it is for: a reload brings both back, so the words never go to whoever shows then.
    for part in ["sessionStorage.setItem(TO, JSON.stringify(to))", "if (box.value.trim()) to = savedTo();", "keep(files.expand(box.value), to)", "scope: to.scope || \"\"", "scope: snap.flags?.scope || \"\""] { assert!(CHAT.contains(part), "{part}"); }
    assert!(CHAT.contains("`To: ${to.label}"));
    // a merge has no filled yes: `danger` leaves both answers plain
    assert!(SHEETS.contains(r#"n === 0 && !ask.danger ? ".btn-primary" : """#));
}
