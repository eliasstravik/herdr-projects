//! The small Markdown reader: headings, paragraphs, lists, tables, quotes, code, links and images. Raw HTML is
//! escaped; links are limited by `safe_url`; nothing here executes or loads anything.
use super::{attr, esc};
use std::sync::OnceLock;

fn safe_url(raw: &str) -> Option<String> {
    let u = raw.trim();
    if u.is_empty() || u.chars().any(|c| (c as u32) < 0x20 || c as u32 == 0x7f) { return None; }
    let low = u.to_lowercase();
    if low.starts_with("http://") || low.starts_with("https://") || low.starts_with("mailto:") { return Some(u.to_string()); }
    if u.starts_with('#') { return Some(u.to_string()); }
    if u.contains(':') || u.starts_with("//") { return None; }
    Some(u.to_string())
}
fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in s.chars() {
        let c = match c { 'á' | 'à' | 'ä' | 'â' => 'a', 'é' | 'è' | 'ë' | 'ê' => 'e', 'í' | 'ì' | 'ï' | 'î' => 'i', 'ó' | 'ò' | 'ö' | 'ô' => 'o', 'ú' | 'ù' | 'ü' | 'û' => 'u', 'ñ' => 'n', _ => c };
        if c.is_alphanumeric() { out.push(c.to_ascii_lowercase()); dash = false; }
        else if !dash && !out.is_empty() { out.push('-'); dash = true; }
    }
    while out.ends_with('-') { out.pop(); }
    out
}

fn inline_text(s: &str) -> String {
    static B: OnceLock<regex::Regex> = OnceLock::new();
    static BI: OnceLock<regex::Regex> = OnceLock::new();
    static I: OnceLock<regex::Regex> = OnceLock::new();
    static S: OnceLock<regex::Regex> = OnceLock::new();
    static ST: OnceLock<regex::Regex> = OnceLock::new();
    let mut out = esc(s);
    let b = B.get_or_init(|| regex::Regex::new(r"\*\*(.+?)\*\*").unwrap());
    out = b.replace_all(&out, "<strong>$1</strong>").into_owned();
    let bi = BI.get_or_init(|| regex::Regex::new(r"__(.+?)__").unwrap());
    out = bi.replace_all(&out, "<strong>$1</strong>").into_owned();
    let st = ST.get_or_init(|| regex::Regex::new(r"~~(.+?)~~").unwrap());
    out = st.replace_all(&out, "<del>$1</del>").into_owned();
    let i = I.get_or_init(|| regex::Regex::new(r"\*([^*\n]+?)\*").unwrap());
    out = i.replace_all(&out, "<em>$1</em>").into_owned();
    let s = S.get_or_init(|| regex::Regex::new(r"_([^_\n]+?)_").unwrap());
    out = s.replace_all(&out, "<em>$1</em>").into_owned();
    out
}

/// Inline Markdown: code spans, links and images are read first; everything else is escaped, then emphasis is added.
fn inline(s: &str) -> String {
    let mut out = String::new();
    let mut plain = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let flush = |out: &mut String, plain: &mut String| { if !plain.is_empty() { out.push_str(&inline_text(plain)); plain.clear(); } };
    while i < chars.len() {
        if chars[i] == '`' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '`' { j += 1; }
            if j < chars.len() {
                flush(&mut out, &mut plain);
                out.push_str(&format!("<code>{}</code>", esc(&chars[i + 1..j].iter().collect::<String>())));
                i = j + 1; continue;
            }
        }
        let image = chars[i] == '!' && chars.get(i + 1) == Some(&'[');
        if chars[i] == '[' || image {
            let open = if image { i + 1 } else { i };
            let mut j = open + 1;
            while j < chars.len() && chars[j] != ']' { j += 1; }
            if j + 1 < chars.len() && chars[j] == ']' && chars[j + 1] == '(' {
                let mut k = j + 2;
                let mut depth = 1;
                while k < chars.len() { if chars[k] == '(' { depth += 1; } else if chars[k] == ')' { depth -= 1; if depth == 0 { break; } } k += 1; }
                if k < chars.len() {
                    let text: String = chars[open + 1..j].iter().collect();
                    let href: String = chars[j + 2..k].iter().collect();
                    let href = href.split_whitespace().next().unwrap_or("");
                    if let Some(url) = safe_url(href) {
                        flush(&mut out, &mut plain);
                        if image { out.push_str(&format!("<img src=\"{}\" alt=\"{}\" loading=\"lazy\">", attr(&url), attr(&text))); }
                        else { out.push_str(&format!("<a href=\"{}\">{}</a>", attr(&url), inline_text(&text))); }
                        i = k + 1; continue;
                    }
                }
            }
        }
        plain.push(chars[i]);
        i += 1;
    }
    flush(&mut out, &mut plain);
    out
}

fn indent_of(line: &str) -> usize { line.chars().take_while(|c| *c == ' ' || *c == '\t').map(|c| if c == '\t' { 4 } else { 1 }).sum() }
fn list_marker(line: &str) -> Option<(bool, usize)> {
    let t = line.trim_start();
    for m in ["- ", "* ", "+ "] { if t.starts_with(m) { return Some((false, indent_of(line) + m.len())); } }
    let d: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !d.is_empty() && t[d.len()..].starts_with(". ") { return Some((true, indent_of(line) + d.len() + 2)); }
    None
}
fn starts_block(line: &str) -> bool {
    let t = line.trim_start();
    t.is_empty() || t.starts_with('#') || t.starts_with("```") || t.starts_with('>') || t == "---" || t == "***" || list_marker(line).is_some() || line.starts_with("    ")
}
fn table_sep(line: &str) -> bool {
    let t = line.trim();
    if !t.contains('-') || !t.contains('|') { return false; }
    t.trim_matches('|').split('|').all(|c| { let c = c.trim(); !c.is_empty() && c.trim_matches(':').chars().all(|x| x == '-') })
}
fn split_row(line: &str) -> Vec<String> { line.trim().trim_matches('|').split('|').map(|c| c.trim().to_string()).collect() }

fn list_block(lines: &[&str], mut i: usize, base: usize) -> (String, usize) {
    let ordered = list_marker(lines[i]).map(|m| m.0).unwrap_or(false);
    let mut items = vec![];
    while i < lines.len() {
        let Some((_, end)) = list_marker(lines[i]) else { break };
        let ind = indent_of(lines[i]);
        if ind < base { break; }
        if ind > base { break; }
        let mut text = lines[i][end.min(lines[i].len())..].trim().to_string();
        i += 1;
        let mut sub = String::new();
        while i < lines.len() {
            if lines[i].trim().is_empty() { i += 1; continue; }
            let ni = indent_of(lines[i]);
            if ni <= base { break; }
            if list_marker(lines[i]).is_some() {
                let (s, n) = list_block(lines, i, ni);
                sub.push_str(&s); i = n;
            } else {
                text.push(' '); text.push_str(lines[i].trim()); i += 1;
            }
        }
        items.push(format!("<li>{}{}</li>", inline(&text), sub));
    }
    let tag = if ordered { "ol" } else { "ul" };
    (format!("<{tag}>{}</{tag}>", items.join("")), i)
}

pub(super) fn blocks(lines: &[&str]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() { i += 1; continue; }
        let t = line.trim_start();
        if t.starts_with("```") {
            let info = t.trim_start_matches("```").trim().to_lowercase();
            let mut code = String::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") { code.push_str(lines[i]); code.push('\n'); i += 1; }
            if i < lines.len() { i += 1; }
            let sketch = info == "html" || info == "svg" || info == "xml";
            if sketch { out.push_str("<p class=\"sketch-note\">Static design sketch from the source. It is shown as text, not run.</p>"); }
            out.push_str(&format!("<pre class=\"code{}\"><code>{}</code></pre>", if sketch { " sketch" } else { "" }, esc(&code)));
            continue;
        }
        if line.starts_with("    ") {
            let mut code = String::new();
            while i < lines.len() && (lines[i].starts_with("    ") || lines[i].trim().is_empty()) { code.push_str(lines[i].strip_prefix("    ").unwrap_or(lines[i])); code.push('\n'); i += 1; }
            out.push_str(&format!("<pre class=\"code\"><code>{}</code></pre>", esc(&code)));
            continue;
        }
        if let Some(rest) = t.strip_prefix('#') {
            let n = rest.chars().take_while(|c| *c == '#').count() + 1;
            if n <= 6 && rest.chars().nth(n - 1) == Some(' ') {
                let text = rest[n..].trim();
                out.push_str(&format!("<h{n} id=\"{}\">{}</h{n}>", attr(&slug(text)), inline(text)));
                i += 1; continue;
            }
        }
        if t == "---" || t == "***" || t == "___" { out.push_str("<hr>"); i += 1; continue; }
        if t.starts_with('>') {
            let mut quote = vec![];
            while i < lines.len() && lines[i].trim_start().starts_with('>') { quote.push(lines[i].trim_start().trim_start_matches('>').trim_start()); i += 1; }
            out.push_str(&format!("<blockquote>{}</blockquote>", blocks(&quote)));
            continue;
        }
        if line.contains('|') && i + 1 < lines.len() && table_sep(lines[i + 1]) {
            let head = split_row(line);
            i += 2;
            let mut rows = vec![];
            while i < lines.len() && lines[i].contains('|') && !lines[i].trim().is_empty() { rows.push(split_row(lines[i])); i += 1; }
            out.push_str("<div class=\"table-wrap\"><table><thead><tr>");
            for h in head { out.push_str(&format!("<th>{}</th>", inline(&h))); }
            out.push_str("</tr></thead><tbody>");
            for r in rows { out.push_str("<tr>"); for c in r { out.push_str(&format!("<td>{}</td>", inline(&c))); } out.push_str("</tr>"); }
            out.push_str("</tbody></table></div>");
            continue;
        }
        if list_marker(line).is_some() {
            let (l, n) = list_block(lines, i, indent_of(line));
            out.push_str(&l); i = n; continue;
        }
        let mut para = vec![line.trim()];
        i += 1;
        while i < lines.len() && !starts_block(lines[i]) && !(lines[i].contains('|') && i + 1 < lines.len() && table_sep(lines[i + 1])) { para.push(lines[i].trim()); i += 1; }
        out.push_str(&format!("<p>{}</p>", inline(&para.join(" "))));
    }
    out
}

