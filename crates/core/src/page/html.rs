//! The page shell: one self-contained HTML file with its CSS, the banner, the readable source and its footer.
use super::{attr, esc, href_path, url_encode, Kind, MARKER};
use super::markdown::blocks;
use std::path::Path;

const CSS: &str = r#"
:root{color-scheme:light dark;--paper:#fff;--ink:#1d2327;--muted:#646970;--line:#dcdcde;--soft:#f6f7f7;--accent:#2271b1;--warn:#fff4d6;--warn-ink:#7a4b00;--code:#f0f0f1}
@media(prefers-color-scheme:dark){:root{--paper:#151a1e;--ink:#e6e8ea;--muted:#a7adb3;--line:#343a40;--soft:#1d242a;--accent:#6cb6ff;--warn:#3a2f16;--warn-ink:#ffd98a;--code:#20272d}}
*{box-sizing:border-box}html{scroll-behavior:smooth}body{margin:0;background:var(--soft);color:var(--ink);font:16px/1.6 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif}main{max-width:920px;margin:0 auto;padding:2.2rem 1rem 4rem}.sheet{background:var(--paper);border:1px solid var(--line);border-radius:10px;padding:clamp(1rem,4vw,2.6rem);box-shadow:0 1px 2px rgba(0,0,0,.04)}.eyebrow{font-size:.75rem;font-weight:700;letter-spacing:.08em;text-transform:uppercase;color:var(--muted)}h1{font-size:clamp(1.6rem,3.5vw,2.2rem);line-height:1.2;margin:.3rem 0 1rem}h2{margin:2rem 0 .6rem;font-size:1.35rem;line-height:1.3}h3{margin:1.4rem 0 .4rem;font-size:1.1rem}h4{margin:1.1rem 0 .3rem}a{color:var(--accent)}p{margin:.7rem 0}ul,ol{padding-left:1.4rem}li{margin:.25rem 0}blockquote{margin:1rem 0;padding:.2rem 1rem;border-left:4px solid var(--line);color:var(--muted)}code{font:13px/1.45 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;background:var(--code);padding:.1rem .3rem;border-radius:4px}pre.code{overflow:auto;background:var(--code);border:1px solid var(--line);border-radius:7px;padding:.9rem;max-height:34rem}pre.code code{background:none;padding:0;white-space:pre}.table-wrap{overflow:auto;margin:1rem 0}table{border-collapse:collapse;width:100%;font-size:.95rem}th,td{border:1px solid var(--line);padding:.45rem .6rem;text-align:left;vertical-align:top}th{background:var(--soft)}img{max-width:100%;height:auto;border:1px solid var(--line);border-radius:6px}.banner{background:var(--warn);color:var(--warn-ink);border:1px solid color-mix(in srgb,var(--warn-ink) 25%,transparent);border-radius:8px;padding:.75rem 1rem;margin:1rem 0 1.4rem;font-size:.95rem}.sketch-note{font-size:.85rem;color:var(--warn-ink);background:var(--warn);border-radius:6px;padding:.4rem .6rem;margin:.9rem 0 -.5rem}.muted{color:var(--muted);font-size:.9rem}.published{margin-top:2rem;border-top:1px solid var(--line);padding-top:1rem;font-size:.92rem}.published ul{padding-left:1.2rem}.published li{margin:.2rem 0}.published .direct{font:12px/1.5 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;word-break:break-all;color:var(--muted)}@media print{body{background:#fff}main{max-width:none;padding:0}.sheet{border:0;box-shadow:none}.banner{break-inside:avoid}}
"#;

/// The page itself: chrome, banner, the rendered source, and the files that belong beside it.
pub fn render(title: &str, source: &str, kind: Kind, markdown: &str, evidence: &[String]) -> String {
    let name = if title.trim().is_empty() { kind.word() } else { title.trim() };
    let source_name = Path::new(source).file_name().map(|x| x.to_string_lossy().into_owned()).unwrap_or_else(|| source.to_string());
    let source_href = href_path(source);
    let ev = evidence.iter().map(|f| format!("<li><a href=\"./{}\">{}</a></li>", url_encode(f), esc(f))).collect::<String>();
    let ev = if ev.is_empty() { "<li class=\"muted\">No evidence files were published with this page.</li>".to_string() } else { ev };
    format!(r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta name="referrer" content="no-referrer">{marker}
<title>{name} — {kind}</title><style>{css}</style></head>
<body><main><div class="sheet"><div class="eyebrow">herdr-projects · {kind}</div><h1>{name}</h1>
<div class="banner">{note}</div>
<article>{body}</article>
<footer class="published"><p><strong>Published by the board.</strong> The source below is the record; this page is its readable view.</p>
<ul><li><a href="./{source_href}">Source Markdown ({source_name})</a><div class="direct">./{source_href}</div></li>
<li>Evidence</li>{ev}</ul>
<p class="muted">Generated from the source when it was written. A missing or unsafe evidence file is not linked.</p></footer>
</div></main></body></html>
"#, marker = MARKER, name = esc(name), kind = kind.word(), css = CSS, note = esc(kind.note()), body = blocks(&markdown.lines().collect::<Vec<_>>()), source_href = attr(&source_href), source_name = esc(&source_name), ev = ev)
}

