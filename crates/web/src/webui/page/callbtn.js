// The call toggle in the web board's chat, beside Hide and Show: a tap starts a voice call with whoever the chat talks
// to (call.js), and the call follows the chat's switch from then on; a tap again hangs up. While a call is on, a strip
// over the chat says how it stands (listening, speaking, muted) and the last things said, with a mute button. A call
// started from the terminal board shows there too (the board says so: `snap.call`), and the toggle hangs it up. Where
// the browser does not let the call be heard, Tap to hear asks for the tap; "Can't hear?" plays it another way (hear.js).
import { h, icon, client, post } from "./dom.js";
import * as call from "./call.js";
import * as hear from "./hear.js";

const STATES = { speaking: "Speaking", listening: "Listening" };
const WHO = { you: "You", voice: "Voice", board: "Board", sent: "Sent", switch: "Board" };
export function callEl({ onEnded } = {}) {
  let other = null, role = "drive";
  const btn = h("button.btn.btn-ghost.sw-call", { type: "button", onclick: () => tap() });
  const mute = h("button.btn.btn-ghost.sw-call-mute", { type: "button", onclick: () => call.mute() });
  const where = h("span.sw-call-where");
  const lines = h("div.sw-call-lines");
  const tapHear = h("button.btn.sw-call-hear", { type: "button", hidden: true, onclick: () => hear.unlock() }, "Tap to hear");
  const fix = h("button.btn.btn-ghost.sw-call-fix", { type: "button", hidden: true, onclick: () => { hear.webAudio(); fix.disabled = true; fix.textContent = "Playing it another way"; } }, "Can't hear?");
  const strip = h("div.sw-call-strip", { hidden: true, "aria-live": "polite" }, h("div.sw-call-top", where, fix, mute), tapHear, lines);
  async function tap() {
    const s = call.state();
    if (s.state !== "idle") return call.end();
    if (other) { const out = await post("voice/end", { why: "hung up on the web board" }); if (out.ok) { other = null; draw(call.state()); onEnded?.(); } return; }
    call.start({ client });
  }
  function draw(s) {
    const mine = s.state !== "idle";
    const on = mine || !!other;
    btn.replaceChildren(icon(on ? "phoneOff" : "phone"), s.state === "starting" ? "Calling…" : on ? "Hang up" : "Call");
    btn.classList.toggle("is-live", on);
    btn.setAttribute("aria-pressed", String(on));
    btn.title = on ? "Hang up the call" : "Call: talk with whoever the chat talks to";
    btn.disabled = role !== "drive" && !mine;
    strip.hidden = !on && !s.note;
    mute.hidden = s.state !== "live";
    const hs = hear.state();
    tapHear.hidden = !(s.state === "live" && hs.blocked); fix.hidden = s.state !== "live" || hs.blocked;
    mute.replaceChildren(icon(s.muted ? "micOff" : "mic"), s.muted ? "Unmute" : "Mute");
    mute.setAttribute("aria-pressed", String(!!s.muted));
    const shown = mine ? s.lines : other ? other.lines || [] : [];
    const state = mine ? (s.state === "starting" ? s.note || "Calling…" : s.muted ? "Muted" : STATES[s.mode] || "On a call") : other ? `On a call from the ${other.from === "terminal" ? "terminal board" : "web board"}` : s.note;
    where.textContent = [state, on && (mine ? s.target : other?.target) ? `with ${mine ? s.target : other.target}` : ""].filter(Boolean).join(" · ");
    lines.replaceChildren(...shown.slice(-2).map((l) => h("p.sw-call-line", h("b", `${WHO[l.by] || l.by}: `), l.text)));
  }
  const stop = call.watch(draw);
  const stopHear = hear.watch(() => draw(call.state()));
  return {
    btn, strip,
    // The board's word on the call of this machine: another page's or the terminal board's shows here too.
    update(snap) { role = snap.role; const c = snap.call, active = call.state().state !== "idle"; other = c && c.on && !(c.mine && active) ? c : null; draw(call.state()); },
    destroy() { stop(); stopHear(); },
  };
}
