//! What herdr's config.toml already holds, in any TOML spelling: sidebar rows of its own, and the key that opens the
//! board. Read as TOML; a file that does not parse is searched for the usual spelling instead.
use toml::{Table, Value};

/// Does the config set herdr's sidebar rows itself? `[ui.sidebar.agents]`, `[ui]` + `sidebar = {…}`, `[ui]` +
/// `sidebar.agents.rows = …` and a top-level `ui.sidebar.agents.rows = …` all do. The plugin's own block is no reason.
pub fn has_sidebar(text: &str) -> bool {
    match text.parse::<Table>() {
        Ok(t) => t.get("ui").and_then(Value::as_table).is_some_and(|ui| ui.contains_key("sidebar")),
        Err(_) => crate::re!(r"(?m)^\s*\[ui\.sidebar").is_match(text),
    }
}
/// The key bound to open the board ("prefix+w"), or "" when none is: in `[[keys.command]]`, in `keys.command = [{…}]`
/// or in `[keys]` + `command = [{…}]`.
pub fn bound_key(text: &str) -> String {
    let Ok(t) = text.parse::<Table>() else { return bound_key_by_eye(text) };
    let cmds = t.get("keys").and_then(Value::as_table).and_then(|k| k.get("command")).and_then(Value::as_array);
    cmds.into_iter().flatten().filter_map(Value::as_table)
        .find(|c| c.get("command").and_then(Value::as_str) == Some("herdr-projects.open"))
        .and_then(|c| c.get("key")).and_then(Value::as_str).unwrap_or_default().to_string()
}
/// A file that does not parse: the binding in its `[[keys.command]]` spelling.
fn bound_key_by_eye(text: &str) -> String {
    let Some(m) = crate::re!(r#"\[\[keys\.command\]\][^\[]*?command\s*=\s*"herdr-projects\.open""#).find(text) else { return String::new() };
    crate::re!(r#"(?m)^\s*key\s*=\s*"([^"]+)""#).captures(m.as_str()).map(|c| c[1].to_string()).unwrap_or_default()
}
