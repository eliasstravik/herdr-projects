// The call in the web board's chat: the Call button beside Hide and Show, which starts a voice call with whoever the
// chat talks to (call.js; the call follows the chat's switch from then on), and the call's own row under that line while
// it rings or is on. The row is one design with the button, which steps aside for it: a dot that says it is live, who
// it is with, where it is held, how long it has run, Mute and End (the way to hang up), and the last thing said. A
// call started from the terminal board shows there too (the board says so: `snap.call`), and End hangs it up. When a
// call ends, the row gives way to one short line ("You hung up · 2 min · 19:20") that fades out after a few seconds:
// nothing about a call that is over stays above the chat. Where the browser does not let the call be heard, Tap to hear
// asks for the tap; "Can't hear?" plays it another way (hear.js).
import { h, icon, client, post } from "./dom.js";
import * as call from "./call.js";
import * as hear from "./hear.js";

const STATES = { speaking: "Speaking", listening: "Listening" };
const WHO = { you: "You", voice: "Voice", board: "Board", sent: "Sent", switch: "Board" };
// How long the line about a call that ended shows (a call that could not start: long enough to read why), and how
// long it takes to fade out at the end of that.
const ENDED_MS = 6000, FAILED_MS = 15000, FADE_MS = 600, LATE_MS = 10000;
const clock = (secs) => `${Math.floor(secs / 60)}:${String(secs % 60).padStart(2, "0")}`;
const lasted = (secs) => (secs <= 0 ? "" : secs < 60 ? `${secs} s` : `${Math.round(secs / 60)} min`);
const hhmm = (at) => new Date(at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
const bare = (name) => (name || "").replace(/^the /, "");

export function callEl({ onEnded } = {}) {
  let other = null, role = "drive", otherSince = 0;
  // The call that ended last, as one line: { text, at, secs, failed }; shown until its time is up.
  let ended = null, seenEnd = 0, fadeT = 0, goneT = 0, tick = 0;
  const btn = h("button.btn.btn-ghost.sw-call", { type: "button", onclick: () => tap() });
  const mute = h("button.btn.btn-ghost.sw-call-mute", { type: "button", onclick: () => call.mute() });
  const hang = h("button.btn.sw-call-end", { type: "button", title: "Hang up the call", onclick: () => tap() }, icon("phoneOff"), "End");
  const dot = h("span.sw-call-dot", { "aria-hidden": "true" });
  const who = h("span.sw-call-who");
  const time = h("span.sw-call-time");
  const where = h("span.sw-call-where");
  const lines = h("div.sw-call-lines");
  const tapHear = h("button.btn.sw-call-hear", { type: "button", hidden: true, onclick: () => hear.unlock() }, "Tap to hear");
  const fix = h("button.btn.btn-ghost.sw-call-fix", { type: "button", hidden: true, onclick: () => { hear.webAudio(); fix.disabled = true; fix.textContent = "Playing it another way"; } }, "Can't hear?");
  const row = h("div.sw-call-row", { hidden: true },
    h("div.sw-call-top", dot, h("div.sw-call-text", h("div.sw-call-name", who, time), h("div.sw-call-sub", where, fix)), h("div.sw-call-acts", mute, hang)),
    tapHear, lines);
  const gone = h("p.sw-call-ended", { hidden: true });
  const strip = h("div.sw-call-strip", { hidden: true, "aria-live": "polite" }, row, gone);
  async function tap() {
    const s = call.state();
    if (s.state !== "idle") return call.end();
    if (other) { const out = await post("voice/end", { why: "hung up on the web board" }); if (out.ok) { over({ text: "You hung up", secs: secsOf(), at: Date.now() }); other = null; draw(call.state()); onEnded?.(); } return; }
    call.start({ client });
  }
  const secsOf = () => (otherSince ? Math.round((Date.now() - otherSince) / 1000) : 0);
  // A call ended: its line shows, fades, and is gone (one that ended before this board was opened, not at all).
  function over(e) {
    clearTimeout(fadeT); clearTimeout(goneT);
    const ms = e.failed ? FAILED_MS : ENDED_MS, left = e.at + ms - Date.now();
    ended = left > 0 ? e : null;
    if (!ended) return;
    gone.classList.remove("is-fading");
    fadeT = setTimeout(() => gone.classList.add("is-fading"), Math.max(0, left - FADE_MS));
    goneT = setTimeout(() => { ended = null; draw(call.state()); }, left);
  }
  function draw(s) {
    if (s.endedAt && s.endedAt !== seenEnd) { seenEnd = s.endedAt; if (s.note) over({ text: s.note, at: s.endedAt, secs: s.secs, failed: s.error }); }
    const mine = s.state !== "idle";
    const on = mine || !!other;
    const ringing = mine ? s.state === "starting" : other?.state === "ringing";
    // The button starts a call; while one rings or is on, the row's End hangs it up and the button steps aside.
    btn.hidden = on;
    btn.replaceChildren(icon("phone"), "Call");
    btn.title = "Call: talk with whoever the chat talks to";
    btn.disabled = role !== "drive";
    hang.disabled = role !== "drive" && !mine;
    hang.replaceChildren(icon("phoneOff"), ringing ? "Cancel" : "End");
    hang.title = ringing ? "Stop calling" : "Hang up the call";
    row.hidden = !on;
    row.dataset.state = ringing ? "ringing" : "live";
    const name = bare(mine ? s.target : other?.target);
    who.textContent = ringing ? "Calling…" : "On call";
    // how it stands, and where it is held when that is not this browser: the terminal board's, on a call helper's computer
    const held = mine ? "" : other?.from === "terminal" ? (other.held ? `on ${other.held}` : "from the terminal board") : "from another browser";
    const how = mine ? (s.state === "starting" ? s.note || "Connecting…" : s.muted ? "Muted" : STATES[s.mode] || "") : "";
    where.textContent = [name, how, held].filter(Boolean).join(" · ");
    since();
    mute.hidden = s.state !== "live";
    const hs = hear.state();
    tapHear.hidden = !(s.state === "live" && hs.blocked); fix.hidden = s.state !== "live" || hs.blocked;
    mute.replaceChildren(icon(s.muted ? "micOff" : "mic"), s.muted ? "Unmute" : "Mute");
    mute.setAttribute("aria-pressed", String(!!s.muted));
    const shown = mine ? s.lines : other ? other.lines || [] : [];
    lines.replaceChildren(...shown.slice(-1).map((l) => h("p.sw-call-line", h("b", `${WHO[l.by] || l.by}: `), l.text)));
    const showEnd = !on && !!ended;
    gone.hidden = !showEnd;
    gone.classList.toggle("is-failed", !!ended?.failed);
    if (showEnd) gone.textContent = [ended.text, lasted(ended.secs), ended.failed ? "" : hhmm(ended.at)].filter(Boolean).join(" · ");
    strip.hidden = !on && !showEnd;
  }
  // How long the call has run, counted every second while it is on.
  function since() {
    const s = call.state(), live = s.state === "live" || (!!other && other.state !== "ringing");
    const from = s.state === "live" ? s.since : otherSince;
    time.hidden = !live || !from;
    if (!time.hidden) time.textContent = clock(Math.max(0, Math.floor((Date.now() - from) / 1000)));
    if (live && !tick) tick = setInterval(since, 1000);
    if (!live && tick) { clearInterval(tick); tick = 0; }
  }
  const stop = call.watch(draw);
  const stopHear = hear.watch(() => draw(call.state()));
  return {
    btn, strip,
    // The board's word on the call of this machine: another page's or the terminal board's shows here too, and its
    // end as this page's own does.
    update(snap) {
      role = snap.role;
      const c = snap.call, active = call.state().state !== "idle";
      // (this page's own call, just ended here, until the board has heard: not somebody else's)
      const late = c && c.mine && Date.now() - seenEnd < LATE_MS;
      const next = c && c.on && !(c.mine && active) && !late ? c : null;
      if (other && !next) over({ text: "Call ended", secs: secsOf(), at: Date.now() });
      const want = next && next.state === "live" ? Date.now() - (next.secs || 0) * 1000 : 0;
      if (!want || !otherSince || Math.abs(want - otherSince) > 2000) otherSince = want;
      other = next;
      draw(call.state());
    },
    destroy() { stop(); stopHear(); clearInterval(tick); clearTimeout(fadeT); clearTimeout(goneT); },
  };
}
