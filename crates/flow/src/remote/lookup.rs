//! The addresses a host name stands for, as this machine's resolver (DNS, Tailscale's MagicDNS, the hosts file) gives
//! them: how a machine named only by its Tailscale name in another machine's herdr list is still found to be this one
//! when its own host name is another (a Mac mini that calls itself `Mac`: remote/sidebar.rs). A look that does not end
//! within a second and a half gives none. The tests stand in for the resolver with a file (`HERDR_PROJECTS_HOSTS`:
//! one host per line, then its addresses), and no real look is made then.
use std::net::{IpAddr, ToSocketAddrs};
use std::time::Duration;

/// The addresses of each host (an address is its own), lowercase and each once, in the order they came.
pub fn addresses(hosts: &[String]) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for h in hosts {
        let h = h.trim().trim_end_matches('.').trim_start_matches('[').trim_end_matches(']').to_lowercase();
        if h.is_empty() || h.starts_with('-') { continue; }
        let found = if let Ok(ip) = h.parse::<IpAddr>() { vec![ip] } else { look(&h) };
        for a in found.iter().filter(|a| !a.is_loopback() && !a.is_unspecified()).map(|a| a.to_string()) { if !out.contains(&a) { out.push(a); } }
    }
    out
}
fn look(host: &str) -> Vec<IpAddr> {
    if let Some(file) = std::env::var_os("HERDR_PROJECTS_HOSTS") {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        return text.lines().map(|l| l.split_whitespace().collect::<Vec<_>>()).filter(|w| w.first().map(|n| n.eq_ignore_ascii_case(host)).unwrap_or(false)).flat_map(|w| w[1..].iter().filter_map(|a| a.parse().ok()).collect::<Vec<_>>()).collect();
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let name = host.to_string();
    std::thread::spawn(move || { let _ = tx.send((name.as_str(), 0).to_socket_addrs().map(|a| a.map(|s| s.ip()).collect::<Vec<_>>()).unwrap_or_default()); });
    rx.recv_timeout(Duration::from_millis(1500)).unwrap_or_default()
}
