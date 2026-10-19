// The page a call from the terminal board opens on the computer you sit at (voice/page.rs): it takes the call's ticket
// from after the `#` (and takes it out of the address at once), starts the call (call.js) and shows how it stands, the
// last things said, Mute and Hang up. Where the browser wants a tap before it may use the microphone or play sound,
// a Start button asks for it. When the call ends it says so, and closes itself where the browser lets it.
/* global document, location, history, window */
import { h } from "./dom.js";
import * as call from "./call.js";

const ticket = location.hash.slice(1);
try { history.replaceState(null, "", location.pathname); } catch {}
const $ = (k) => document.querySelector(`[data-${k}]`);
const WHO = { you: "You", voice: "Voice", board: "Board" };
let began = false;
const go = () => { began = true; $("start").hidden = true; call.start({ ticket }); };
$("start").addEventListener("click", go);
$("mute").addEventListener("click", () => call.mute());
$("end").addEventListener("click", () => call.end());
call.watch((s) => {
  const live = s.state === "live";
  $("who").textContent = live || s.state === "starting" ? (s.target ? `On a call with ${s.target}` : "Calling…") : began ? "Call ended" : "Call";
  $("state").textContent = s.state === "starting" ? s.note || "Connecting…" : live ? (s.muted ? "Muted" : s.mode === "speaking" ? "Speaking" : "Listening") : s.note || "";
  $("mute").hidden = !live; $("mute").textContent = s.muted ? "Unmute" : "Mute";
  $("end").hidden = !live && s.state !== "starting";
  $("lines").replaceChildren(...s.lines.slice(-4).map((l) => h("p", h("b", `${WHO[l.by] || l.by}: `), l.text)));
  // The microphone or the sound wanted a tap: the button asks for it, with the same ticket (it was not used yet).
  if (s.state === "idle" && began && /microphone was not allowed/.test(s.note || "")) $("start").hidden = false;
  if (s.state === "idle" && began && /ended|hung up/i.test(s.note || "")) setTimeout(() => { try { window.close(); } catch {} }, 1500);
});
if (!ticket) { $("who").textContent = "No call here"; $("state").textContent = "Press the call key on the board to start a call."; }
else go();
