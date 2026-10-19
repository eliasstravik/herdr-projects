//! `herdr-projects voice`: how calls stand on this machine (where the ElevenLabs key was found, never the key itself;
//! the front door's link a call from the terminal board opens; whether this computer's call helper runs, and for which
//! machines; a call on now), and `voice end` to hang one up. `voice helper` and `voice listen` are the helper's own
//! (helper.rs, listen.rs), run in the background.
use crate::json::*;
use crate::voice::{call, key};

pub fn main(args: &[String]) -> (Value, String) {
    if args.first().map(|s| s.as_str()) == Some("end") {
        let on = call::get().is_some();
        call::end("hung up from the command line");
        return (json!({ "ended": on }), if on { "The call was hung up.".into() } else { "No call was on.".into() });
    }
    let k = key::find();
    let link = super::ring::door_link();
    let machines: Vec<String> = crate::remote::machines().into_iter().map(|m| m.label).collect();
    let helper = super::helper::alive();
    let c = call::get();
    let mut lines = vec![
        match &k { Some(k) => format!("ElevenLabs key: found ({})", k.from), None => format!("ElevenLabs key: none. {}", key::missing()) },
        match &link { Some(l) => format!("Call page: {l}/herdr-projects/call (a call from the terminal board opens it, with its ticket)"), None => "Call page: none here (the board's front door has no private link: herdr-projects web-ui link). The web board can still call when its page is open.".into() },
        if machines.is_empty() { "Call helper: not needed here (herdr has no saved machines: this is where the board runs, or you sit here)".into() } else { format!("Call helper: {} for {}", if helper { "running" } else if super::helper::wanted() { "not running (the keeper starts it)" } else { "off (voice.helper is none)" }, machines.join(", ")) },
    ];
    lines.push(match &c { Some(c) => format!("Call: {} with {} ({})", c.s("state"), call::target(c).name, if c.s("owner") == "terminal" { "from the terminal board" } else { "from the web board" }), None => "Call: none on now".into() });
    (json!({ "key": k.map(|k| k.from), "link": link, "helper": helper, "machines": machines, "call": c.map(|c| json!({ "id": c.s("id"), "state": c.s("state"), "owner": c.s("owner") })) }), lines.join("\n"))
}
