//! The names this machine goes by, as another machine's herdr list of machines may name it (remote/sidebar.rs): its
//! host name (whole; a `.local` one also as the bare name it is the local network's form of), and the addresses of
//! its network interfaces (a Tailscale address too: Tailscale is an interface), never a loopback one. Read once,
//! without starting a program.
use std::net::IpAddr;

/// This machine's names, lowercase: read once, the first time they are asked for.
pub fn own_names() -> Vec<String> {
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NAMES.get_or_init(|| { let mut v = host_names(&super::check::hostname()); v.extend(addresses().iter().map(|a| a.to_string())); v }).clone()
}
/// A host name as names: the whole of it, and for `name.local` (the local network's form of a bare name, as a Mac
/// names itself) also `name`. No other part of a name is taken for a name: `build.prod.example` is not `build`.
pub fn host_names(host: &str) -> Vec<String> {
    let h = host.trim().trim_end_matches('.').to_lowercase();
    if h.is_empty() || h == "localhost" { return vec![]; }
    match h.strip_suffix(".local").filter(|b| !b.is_empty() && !b.contains('.')) { Some(b) => vec![h.clone(), b.to_string()], None => vec![h] }
}
/// The addresses of this machine's network interfaces, loopback ones left out. Not looked up on Windows: the machine is
/// known by its name there.
#[cfg(windows)]
fn addresses() -> Vec<IpAddr> { vec![] }
/// The addresses of this machine's network interfaces, loopback ones left out.
#[cfg(unix)]
fn addresses() -> Vec<IpAddr> {
    let mut out = vec![];
    let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut list) } != 0 { return out; }
    let mut at = list;
    while !at.is_null() {
        let a = unsafe { &*at };
        if !a.ifa_addr.is_null() {
            let family = unsafe { (*a.ifa_addr).sa_family } as i32;
            let ip = if family == libc::AF_INET { Some(IpAddr::from(u32::from_be(unsafe { (*(a.ifa_addr as *const libc::sockaddr_in)).sin_addr.s_addr }).to_be_bytes())) }
                else if family == libc::AF_INET6 { Some(IpAddr::from(unsafe { (*(a.ifa_addr as *const libc::sockaddr_in6)).sin6_addr.s6_addr })) } else { None };
            if let Some(ip) = ip.filter(|ip| !ip.is_loopback() && !ip.is_unspecified()) { if !out.contains(&ip) { out.push(ip); } }
        }
        at = a.ifa_next;
    }
    unsafe { libc::freeifaddrs(list) };
    out
}
