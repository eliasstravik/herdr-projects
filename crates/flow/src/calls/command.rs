//! `herdr-projects voice`: how calls stand on this machine (where the ElevenLabs key was found, never the key itself;
//! whether this computer's call helper runs, and for which machines; how the last call sounded; a call on now), and
//! `voice end` to hang one up, and `voice test` to try this computer's sound for a call (a short tone out, a moment of
//! the microphone in, and whether the echo is cancelled). `voice helper` and `voice listen` are the helper's own
//! (helper.rs, listen.rs), run in the background. With `voice.calls off` it says only that, and how to turn them on.
use crate::json::*;
use crate::voice::{call, key};

/// The sound a call would have here: a tone played, the microphone listened to, the echo cancelling looked for.
pub fn test_sound() -> (Value, String) {
    let mut a = match super::native::audio::open() { Ok(a) => a, Err(e) => return (json!({ "ok": false, "msg": e }), format!("No sound for a call here: {e}")) };
    let tone: Vec<u8> = (0..8000).flat_map(|i| ((((i as f64) * 440.0 * std::f64::consts::TAU / 16000.0).sin() * 8000.0) as i16).to_le_bytes()).collect();
    a.play(&tone);
    let mut peak = 0u16; let mut loud = 0f64;
    let until = std::time::Instant::now() + std::time::Duration::from_millis(1500);
    while std::time::Instant::now() < until { for c in a.mic() { loud = loud.max(super::native::level::rms(&c)); for s in c.chunks(2) { if s.len() == 2 { peak = peak.max(i16::from_le_bytes([s[0], s[1]]).unsigned_abs()); } } } std::thread::sleep(std::time::Duration::from_millis(20)); }
    let (how, aec) = (a.how(), a.aec());
    a.stop();
    let mic = if peak > 300 { format!("it hears sound (peak {peak}, loudest {:.0} rms{})", loud, if loud < 300.0 { ": quiet, speak up or move closer" } else { "" }) } else { "it heard nothing in a second and a half (no microphone, or no permission: on macOS, Privacy & Security, Microphone, the terminal herdr runs in)".to_string() };
    (json!({ "ok": true, "how": how, "aec": aec, "peak": peak }), format!("Sound for a call here: {how}.\nA tone was played: did you hear it?\nMicrophone: {mic}.\n{}", if aec { "Echo cancelling: on (speakers are fine)." } else { "Echo cancelling: none here, so use headphones on a call." }))
}
pub fn main(args: &[String]) -> (Value, String) {
    if args.first().map(|s| s.as_str()) == Some("test") { return test_sound(); }
    if args.first().map(|s| s.as_str()) == Some("end") {
        let on = call::get().is_some();
        call::end("hung up from the command line");
        return (json!({ "ended": on }), if on { "The call was hung up.".into() } else { "No call was on.".into() });
    }
    if !crate::voice::calls_on() {
        let off = format!("{}: no call rings, the call helper does not run, and nothing contacts ElevenLabs.\nTurn them on: herdr-projects set voice.calls on", crate::voice::OFF);
        return (json!({ "calls": "off" }), off);
    }
    let k = key::find();
    let machines: Vec<String> = crate::remote::machines().into_iter().map(|m| m.label).collect();
    let helper = super::helper::alive();
    let c = call::get();
    let mut lines = vec![
        match &k { Some(k) => format!("ElevenLabs key: found ({})", k.from), None => format!("ElevenLabs key: none. {}", key::missing()) },
        if machines.is_empty() { "Call helper: not needed here (herdr has no saved machines: this is where the board runs, or you sit here)".into() } else { format!("Call helper: {} for {}", if helper { "running" } else if super::helper::wanted() { "not running (the keeper starts it)" } else { "off (voice.helper is none)" }, machines.join(", ")) },
    ];
    // How the page holding the last call heard it (hear.js): whether sound arrived, played, or was not allowed.
    if let Some(l) = call::last().filter(|l| l.has("audio")) {
        let a = l.at("audio");
        let how = if a.s("via") == "native" { format!("held by {} with {}; the voice agent's sound peaked at {}{}", a.s("agent"), a.s("how"), a.t("peak"), if a.b("aec") { "" } else { " (headphones needed: no echo cancelling there)" }) }
            else if a.b("blocked") { "the browser did not let it play sound (a tap on Tap to hear fixes that)".to_string() }
            else if a.i("elements") == 0 { "no sound of the voice agent reached the page".into() }
            else if a.f("peak") < 0.001 { format!("the voice agent's sound reached the page but was silent ({} playing)", a.i("playing")) }
            else { format!("sound reached the page (peak {}) and played through {}", a.t("peak"), if a.s("via") == "webaudio" { "the page's audio context" } else { "an audio element" }) };
        lines.push(if a.s("via") == "native" { format!("Sound, last call ({}): {how}", l.s("id")) } else { format!("Sound, last call ({}): {how}; audio context {}; {}", l.s("id"), a.s("ctx"), a.s("agent")) });
    }
    lines.push(match &c { Some(c) => format!("Call: {} with {} ({})", c.s("state"), call::target(c).name, if call::from_terminal(c) { "from the terminal board" } else { "from the web board" }), None => "Call: none on now".into() });
    (json!({ "key": k.map(|k| k.from), "helper": helper, "machines": machines, "call": c.map(|c| json!({ "id": c.s("id"), "state": c.s("state"), "owner": c.s("owner") })) }), lines.join("\n"))
}
