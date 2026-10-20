//! Where the web board's call reaches ElevenLabs: the web UI's own policy, with the call's hosts added where it says
//! where a page may connect, so the board's chat can hold a call. Nothing else in it changes. With `voice.calls off` the
//! policy stays the web UI's own: the page may not reach ElevenLabs.

/// Where the call's audio goes: LiveKit at ElevenLabs (signalling over https and wss), and its API.
pub const HOSTS: &str = "https://api.elevenlabs.io wss://livekit.rtc.elevenlabs.io https://livekit.rtc.elevenlabs.io";
pub fn widen(policy: &str) -> String {
    if !crate::voice::calls_on() { return policy.into(); }
    if policy.contains("livekit.rtc.elevenlabs.io") { return policy.into(); }
    let parts: Vec<String> = policy.split(';').map(|d| d.trim().to_string()).filter(|d| !d.is_empty()).collect();
    if parts.iter().any(|d| d.starts_with("connect-src")) {
        return parts.iter().map(|d| if d.starts_with("connect-src") { format!("{d} {HOSTS}") } else { d.clone() }).collect::<Vec<_>>().join("; ");
    }
    // No word on connecting: it follows default-src, so connect-src is that, and the call's hosts.
    match parts.iter().find(|d| d.starts_with("default-src")) {
        Some(d) => { let mut out = parts.clone(); out.push(format!("connect-src{} {HOSTS}", &d["default-src".len()..])); out.join("; ") }
        None => policy.into(),
    }
}
