//! The herdr web UI plugin, Ovis (`devswha.ovis`; `devswha.herdr-web-ui` before its rename): whether it is installed in
//! this herdr, under which of its two ids, and the address its server listens on. Read from herdr and from the web UI's
//! own files, never written: its settings (`PORT`, `HOST` in the `env` or `.env` of its config folder) and the port a
//! start of it settled on (`plugin-port` in its state folder). Checked on herdr-web-ui 0.4.8 and on Ovis 0.4.13 under
//! its new id (its main, `22104941`).
use crate::herdr::Herdr;
use crate::json::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The web UI's plugin id since its rename to Ovis.
pub const OVIS: &str = "devswha.ovis";
/// Its id before the rename (herdr-web-ui, and Ovis up to 0.4.13): an install from then keeps it until Ovis's own
/// installer moves it to the new one.
pub const OLD: &str = "devswha.herdr-web-ui";
/// The ids it is looked for under, the new one first: herdr may list both while the installer moves it.
pub const WEB_UI_IDS: [&str; 2] = [OVIS, OLD];
pub const DEFAULT_PORT: u16 = 7317;
/// `plugin`: the id it was found under, and `version` its version there (both "" for one named by hand).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct WebUi { pub host: String, pub port: u16, pub socket: String, pub plugin: String, pub version: String, pub named: bool }
fn env_files(dir: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for name in ["env", ".env"] {
        let Ok(text) = std::fs::read_to_string(Path::new(dir).join(name)) else { continue };
        for line in text.lines() {
            let t = line.trim();
            let Some(eq) = t.find('=') else { continue };
            if t.starts_with('#') || eq == 0 { continue; }
            vars.insert(t[..eq].trim().to_string(), t[eq + 1..].trim().trim_matches(|c| c == '"' || c == '\'').to_string());
        }
    }
    vars
}
/// Its state folder, as the code under each id finds it. Ovis: `OVIS_STATE_DIR`, else `ovis` in the config folder, but the
/// old `herdr-web-ui` there while that still holds state of its own (Ovis moves it over, and keeps using it where it
/// could not). Under the old id: `HERDR_WEB_STATE_DIR`, else `herdr-web-ui`.
fn state_dir(vars: &HashMap<String, String>, plugin: &str) -> PathBuf {
    let var = |k: &str| vars.get(k).cloned().or_else(|| std::env::var(k).ok()).filter(|v| !v.is_empty()).map(PathBuf::from);
    let config = var("XDG_CONFIG_HOME").unwrap_or_else(|| crate::store::files::user_home().join(".config"));
    let (old, new) = (config.join("herdr-web-ui"), config.join("ovis"));
    if plugin == OLD { return var("HERDR_WEB_STATE_DIR").unwrap_or(old); }
    var("OVIS_STATE_DIR").unwrap_or_else(|| if holds_own_state(&old, &new) { old } else { new })
}
/// The old state folder is one of its own: not empty, and not the new one under its old name (the link Ovis leaves).
fn holds_own_state(old: &Path, new: &Path) -> bool {
    let full = std::fs::read_dir(old).map(|mut d| d.next().is_some()).unwrap_or(false);
    full && (!new.is_dir() || std::fs::canonicalize(old).ok() != std::fs::canonicalize(new).ok())
}
/// Where the web UI listens and which herdr it shows, from its plugin's config folder and the id it was found under.
pub fn address_from(config_dir: &str, plugin: &str) -> WebUi {
    let vars = if config_dir.is_empty() { Default::default() } else { env_files(config_dir) };
    let host = vars.get("HOST").cloned().unwrap_or_else(|| "127.0.0.1".into());
    // Its herdr, as the web UI's own start works it out: the startup socket herdr hands every plugin wins.
    let socket = vars.get("HERDR_SOCKET_PATH").cloned().or_else(|| std::env::var("HERDR_SOCKET_PATH").ok()).unwrap_or_else(|| vars.get("HERDR_SOCKET").cloned().or_else(|| std::env::var("HERDR_SOCKET").ok()).unwrap_or_default());
    let found = WebUi { host, socket, plugin: plugin.into(), ..Default::default() };
    if let Some(p) = vars.get("PORT").and_then(|p| p.parse::<u16>().ok()).filter(|p| *p > 0) { return WebUi { port: p, ..found }; }
    let saved = std::fs::read_to_string(state_dir(&vars, plugin).join("plugin-port")).ok().and_then(|s| s.trim().parse::<u16>().ok()).filter(|p| *p > 0);
    WebUi { port: saved.unwrap_or(DEFAULT_PORT), ..found }
}
/// A web UI named by hand (`HERDR_PROJECTS_WEBUI=17431`, or `host:port`): one herdr does not list. `off`: none.
fn named(v: &str) -> Option<Option<WebUi>> {
    let v = v.trim();
    if v.is_empty() { return None; }
    if v.eq_ignore_ascii_case("off") { return Some(None); }
    let (host, port) = match v.rfind(':') { Some(at) => (&v[..at], &v[at + 1..]), None => ("127.0.0.1", v) };
    Some(port.parse::<u16>().ok().filter(|p| *p > 0).map(|port| WebUi { host: host.into(), port, named: true, ..Default::default() }))
}
static KEPT: Mutex<(i64, Option<WebUi>)> = Mutex::new((0, None));
/// The web UI's address, or None when there is none to stand in front of. Asking herdr starts a program, so the answer
/// is kept for a few seconds.
pub fn web_ui(fresh: bool) -> Option<WebUi> {
    let now = crate::clock::now();
    { let k = KEPT.lock().unwrap(); if !fresh && k.0 > 0 && now - k.0 < 15000 { return k.1.clone(); } }
    let v = look();
    *KEPT.lock().unwrap() = (now, v.clone());
    v
}
fn look() -> Option<WebUi> {
    // Off by the setting, or taken out with `remove-extras` (until `web-ui link` asks for it again).
    if crate::store::settings::lookup::setting("board.webUi", None) == json!("none") || crate::extras::extras().s("webUi") == "removed" { return None; }
    if let Some(by_hand) = named(&std::env::var("HERDR_PROJECTS_WEBUI").unwrap_or_default()) { return by_hand; }
    let h = Herdr::new(None);
    let plugins = h.plugins();
    let p = WEB_UI_IDS.iter().find_map(|id| plugins.iter().find(|p| p.s("plugin_id") == *id && p.at("enabled") != &json!(false)))?;
    let mut w = address_from(&h.plugin_config_dir(p.s("plugin_id")), p.s("plugin_id"));
    w.version = p.t("version");
    Some(w)
}
