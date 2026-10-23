//! The web UI's own machines (its "PCs": this one and the ones added in its Settings → Remote PCs) and the board's
//! (herdr's saved machines, remote.rs): which is which. The board needs it twice: to open another machine's agent in
//! the web UI's own view of that pane (`?machine=<its PC>`), and to show the board of the machine the web UI is on.
//! Nothing is written: the front door asks the web UI, with the caller's own headers, what PCs it has
//! (`GET /api/machines`, checked on herdr-web-ui 0.4.8), keeps the answer a few seconds, and passes on what is needed.
//! Each PC's answer also has the panes the web UI knows there (`snapshot.panes`, checked on Ovis 0.4.13): a link to a
//! pane it does not know yet is dropped by its page (webui/going.rs waits for it).
use super::proxy::{ask, caller_headers, Head};
use crate::json::*;
use crate::remote::Machine;

pub const PCS: &str = "/api/machines";
/// `panes`: the panes the web UI knows on that PC (None: it sent no list of them).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pc { pub id: String, pub name: String, pub local: bool, pub profile: String, pub to: String, pub port: u16, pub session: String, pub connected: bool, pub state: String, pub panes: Option<Vec<String>> }
/// What the web UI answered, as the few things the board needs of each PC; None when it is no such answer (an older
/// web UI, a bridge that manages no PCs, a caller it does not let in).
pub fn read_pcs(text: &str) -> Option<Vec<Pc>> {
    let j: Value = serde_json::from_str(text).ok()?;
    let list = j.at("machines").as_array()?;
    Some(list.iter().filter(|m| m.at("id").as_str().map(|s| !s.is_empty()).unwrap_or(false)).map(|m| {
        let off = m.at("enabled") == &json!(false);
        let port = m.at("target").at("port");
        Pc { id: m.t("id"), name: if m.b("name") { m.t("name") } else { m.t("id") }, local: m.s("kind") == "local" || m.s("id") == "local", profile: m.t("herdr_profile_id"),
            to: m.at("target").t("destination"), port: port.as_u64().or_else(|| port.as_str().and_then(|p| p.parse().ok())).unwrap_or(0) as u16, session: m.at("target").t("session"),
            connected: !off && m.s("state") == "connected", state: if off { "turned off".into() } else { m.t("state") },
            panes: m.at("snapshot").at("panes").as_array().map(|l| l.iter().map(|p| p.t("pane_id")).filter(|p| !p.is_empty()).collect()) }
    }).collect())
}
static KNOWN: std::sync::Mutex<(i64, Option<Vec<Pc>>)> = std::sync::Mutex::new((0, None));
pub fn forget() { *KNOWN.lock().unwrap() = (0, None); }
/// Asks the web UI (the answer carries every PC's whole list of panes, so it may be long), kept five seconds.
pub fn ask_pcs(h: &Head, to: &(String, u16)) -> Option<Vec<Pc>> {
    let now = crate::clock::now();
    { let k = KNOWN.lock().unwrap(); if now - k.0 < 5000 { return k.1.clone(); } }
    let list = ask(&to.0, to.1, "GET", PCS, &caller_headers(h), 16 << 20, 8000).filter(|a| a.status == 200).and_then(|a| read_pcs(&String::from_utf8_lossy(&a.body)));
    *KNOWN.lock().unwrap() = (crate::clock::now(), list.clone());
    list
}
/// The same, asked now: what was kept is not used (a pane just reopened may not be in it).
pub fn ask_pcs_now(h: &Head, to: &(String, u16)) -> Option<Vec<Pc>> { forget(); ask_pcs(h, to) }
/// An SSH address as its parts: `user@host`, `host`, or herdr's `ssh://user@host:port`. An SSH alias is a host.
pub fn address(target: &str, port: u16) -> (String, String, u16) {
    if let Some(c) = crate::re!(r"^ssh://(?:([^@/]+)@)?(\[[^\]]+\]|[^:/]+)(?::(\d+))?$").captures(target) {
        let p = c.get(3).and_then(|x| x.as_str().parse().ok()).filter(|p| *p > 0).unwrap_or(port);
        return (c.get(1).map(|x| x.as_str().to_string()).unwrap_or_default(), c[2].to_lowercase(), p);
    }
    match crate::re!(r"^(?:([^@]+)@)?(.+)$").captures(target) {
        Some(c) => (c.get(1).map(|x| x.as_str().to_string()).unwrap_or_default(), c[2].to_lowercase(), port),
        None => (String::new(), String::new(), port),
    }
}
fn session(s: &str) -> &str { if s == "default" { "" } else { s } }
/// Is this PC of the web UI that machine of the board? By herdr's own id for the saved machine when the web UI names it
/// (it does for a PC it took from `herdr machine list`); else by where it connects: the same host, port and herdr
/// session, and the same user where both say one. A port left out is not taken for 22: SSH's own config decides it.
pub fn same(m: &Machine, pc: &Pc) -> bool {
    if pc.local { return false; }
    if !pc.profile.is_empty() && !m.id.is_empty() { return pc.profile == m.id; }
    let (a, b) = (address(&m.target, 0), address(&pc.to, pc.port));
    !a.1.is_empty() && a.1 == b.1 && a.2 == b.2 && (a.0.is_empty() || b.0.is_empty() || a.0 == b.0) && session(&m.session) == session(&pc.session)
}
pub fn pc_of(m: &Machine, pcs: &[Pc]) -> Option<Pc> { pcs.iter().find(|pc| same(m, pc)).cloned() }
pub fn machine_of(pc: &Pc, ms: &[Machine]) -> Option<Machine> { ms.iter().find(|m| same(m, pc)).cloned() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pcs_match_machines() {
        let pcs = read_pcs(r#"{"machines":[{"id":"local","kind":"local","state":"connected"},{"id":"pc-2","name":"Box","state":"connected","target":{"destination":"me@Box.lan","port":2222}},{"id":"pc-3","herdr_profile_id":"m1","enabled":false}]}"#).unwrap();
        assert_eq!(pcs.len(), 3);
        assert!(pcs[0].local && !pcs[2].connected && pcs[2].state == "turned off");
        let m = Machine { id: String::new(), target: "ssh://me@box.lan:2222".into(), session: "default".into(), label: "box".into() };
        assert_eq!(pc_of(&m, &pcs).map(|p| p.id), Some("pc-2".into()));
        let by_id = Machine { id: "m1".into(), ..Default::default() };
        assert_eq!(pc_of(&by_id, &pcs).map(|p| p.id), Some("pc-3".into()));
        assert_eq!(address("box", 0), (String::new(), "box".into(), 0));
        assert!(read_pcs("{}").is_none());
    }
}
