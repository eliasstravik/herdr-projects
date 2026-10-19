// A voice call held in this page: a live conversation with the board's ElevenLabs agent over WebRTC, with the
// browser's echo cancelling, which talks with you and sends what should go to the agent the chat talks to with its
// tools (run by the board: the front door's voice calls, voice/api.rs). The front door gives a short-lived token, never
// the API key. Your words and the voice agent's are told to the board as they are said, so the chats show them; what to
// read back (an agent's answer, a task that needs you) is asked for every second and a half and said only in a pause:
// never while you talk, never over the voice agent. One call per page, kept while the board is closed and opened again.
// Whether the voice agent can be heard is hear.js's: it is reported to the board with each fifth question.
/* global window, document, navigator, fetch, performance */
import * as hear from "./hear.js";
const API = "/herdr-projects/api/voice";
const TOOLS = ["send_message", "switch_to", "board_status", "latest_reply", "quiet", "end_call"];
// A pause: nobody spoke for this long (you, by the voice service's own speech detector; the voice agent, by its mode).
const QUIET_MS = 1400, FEED_MS = 1500, VOICE = 0.5;

const subs = new Set();
// `since`: when the call was answered here; `endedAt`, `secs`, `error`: when the last one ended, how long it ran and
// whether it never started (its `note` says why). How long that note shows is callbtn.js's.
let st = { state: "idle", mode: "", muted: false, target: "", note: "", lines: [], since: 0, endedAt: 0, secs: 0, error: false };
let hush = false; // told to be quiet: updates about other tasks are dropped, also ones already on their way
let conv = null, key = "", timer = null, queue = [], heardAt = 0, spokeAt = 0, waiting = false, lock = null, cancelled = false;
// What was said, told to the board one after the other (tried three times): a tool waits for it, so your words are
// there before it sends. Words of yours that never got there stop every message (`lost`) until the voice agent was
// told to ask you to say the whole request again ("asked") and words of yours spoken after that reached the board.
// Words belong to the call they were said on: they go with that call's key only, and a call that ended takes its
// words, its tries and its losses with it (a new call starts with none).
let told = Promise.resolve(), lost = "";
const tell = (who, text) => { const k = key; told = told.then(async () => {
  for (let n = 0; n < 3; n++) { if (key !== k || !k) return; if ((await send("heard", { who, text }, "POST", k)).ok) { if (key === k && who === "user" && lost === "asked") lost = ""; return; } }
  if (key === k && who === "user") { lost = "lost"; say("board", "Those words did not reach the board: say the whole request again."); }
}); };
const fresh = () => { told = Promise.resolve(); lost = ""; };
const set = (patch) => { st = { ...st, ...patch }; for (const f of subs) { try { f(st); } catch {} } };
export const state = () => st;
export const watch = (f) => { subs.add(f); f(st); return () => subs.delete(f); };
// The voice's expression tags ("[neutral]") are for how it speaks, never shown (the board keeps its lines without them):
// lower-case words in square brackets that start a sentence, the last followed by a capital or nothing, as voice/words.rs.
const untag = (t) => t.split(/(?<=[.!?…]\s+)/).map((p) => {
  for (let m; (m = p.match(/^\s*\[\p{Ll}[\p{Ll} -]{0,39}\]\s*/u)); ) {
    const after = p.slice(m[0].length);
    if (after === "" || after.startsWith("[") || /^\p{Lu}/u.test(after)) p = after; else break;
  }
  return p;
}).join("").replace(/\s+/g, " ").trim();
const say = (by, text) => set({ lines: [...st.lines, { by, text: by === "voice" ? untag(text) : text }].slice(-6) });

async function send(what, body, method = "POST", k = key) {
  try {
    const r = await fetch(`${API}/${what}`, { method, credentials: "same-origin", cache: "no-store", keepalive: what === "end", headers: { "content-type": "application/json", "x-herdr-projects": "1", "x-herdr-projects-call": k }, body: method === "POST" ? JSON.stringify(body || {}) : undefined });
    const j = await r.json().catch(() => ({}));
    return { ...j, ok: r.ok && j.ok !== false, status: r.status };
  } catch { return { ok: false, status: 0, msg: "No connection." }; }
}
// Why a call could not start, in plain words.
function why(e) {
  const n = e && e.name;
  if (n === "NotAllowedError" || n === "SecurityError") return "The microphone was not allowed. Allow it for this page, then call again.";
  if (n === "NotFoundError") return "No microphone was found.";
  return (e && e.message) || "The call could not start.";
}
// Start a call: from the web board (`client`, its page's own board) or from the terminal board (`ticket`).
export async function start(how) {
  if (conv || st.state === "starting") return;
  if (!window.isSecureContext) return set({ state: "idle", note: "A call needs the board's private https link.", endedAt: Date.now(), secs: 0, error: true });
  set({ state: "starting", note: "Asking for the microphone…", lines: [], since: 0, endedAt: 0, secs: 0, error: false });
  cancelled = false; fresh(); hush = false;
  // Hung up while it was still connecting: it stops at the next step, and the board is told.
  const stop = () => { if (!cancelled) return false; if (key) send("end", { why: "hung up while connecting" }); key = ""; set({ state: "idle", note: "Call cancelled", mode: "", endedAt: Date.now(), secs: 0 }); return true; };
  try {
    const mic = await navigator.mediaDevices.getUserMedia({ audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true } });
    for (const t of mic.getTracks()) t.stop();
    if (stop()) return;
    set({ note: "Connecting…" });
    const r = await fetch(`${API}/start`, { method: "POST", credentials: "same-origin", headers: { "content-type": "application/json", "x-herdr-projects": "1" }, body: JSON.stringify(how) });
    const out = await r.json().catch(() => ({}));
    if (!r.ok || !out.token) throw new Error(out.msg || "The call could not start.");
    key = out.key;
    if (stop()) return;
    const { Conversation } = await import("./vendor/elevenlabs.js");
    // This call's tools: each runs with this call's key only. One still waiting when the call ended does nothing in
    // the next call.
    const mine = out.key;
    const clientTools = Object.fromEntries(TOOLS.map((name) => [name, async (args) => {
      await told;
      if (key !== mine) return "Not sent: this call has ended.";
      // Lost words: this message does not go, and the human is asked to say all of it again; their next words count.
      if (name === "send_message" && lost) { lost = "asked"; return "Not sent: some of the human's words did not reach the board. Ask them to say their whole request again, then send it."; }
      const res = await send("tool", { name, args: args || {} }, "POST", mine);
      if (name === "end_call") setTimeout(() => end("said goodbye"), 2500);
      // quiet: updates about other tasks waiting for a pause are dropped too, not only the ones to come
      if (name === "quiet" && res.ok) { hush = !!args?.on; if (hush) queue = queue.filter((i) => i.kind !== "alert"); }
      if (name === "switch_to" || name === "send_message") feed();
      return res.ok ? String(res.result || "") : res.msg || "The board did not answer.";
    }]));
    let dropped = false; // the voice service let go before the call was set up here
    const started = await Conversation.startSession({
      conversationToken: out.token, connectionType: "webrtc", workletPaths: { rawAudioProcessor: out.worklet },
      dynamicVariables: { target: out.target }, clientTools,
      onConnect: () => { set({ state: "live", note: "", target: out.target, since: Date.now() }); hear.start(); },
      onDisconnect: () => { if (conv) finish("Call ended", "the voice service ended it"); else dropped = true; },
      onError: (m) => set({ note: typeof m === "string" ? m : "The call had a problem." }),
      onModeChange: ({ mode }) => { set({ mode }); if (mode === "speaking") spokeAt = performance.now(); },
      onVadScore: ({ vadScore }) => { if (vadScore > VOICE) heardAt = performance.now(); },
      onMessage: ({ message, role, source }) => {
        const by = role === "agent" || source === "ai" ? "agent" : "user";
        if (by === "user") heardAt = performance.now();
        if (!message || /^\[board\]/.test(message)) return;
        say(by === "agent" ? "voice" : "you", message);
        tell(by, message);
      },
    });
    conv = started;
    if (dropped) return finish("Call ended", "the voice service ended it");
    if (cancelled) return end();
    set({ state: "live", target: out.target });
    try { lock = await navigator.wakeLock?.request("screen"); } catch {}
    timer = setInterval(feed, FEED_MS);
  } catch (e) {
    const note = why(e);
    if (key) send("end", { why: "could not start" });
    conv = null; key = "";
    set({ state: "idle", note, mode: "", endedAt: Date.now(), secs: 0, error: true });
  }
}
// Ask the board what to read back, and read the next of it in a pause.
let asks = 0;
async function feed() {
  if (!conv || waiting) return;
  waiting = true;
  const k = key; // what comes back is this call's, and nothing of the next one's
  if (asks++ % 5 === 0) send("audio", hear.report());
  const out = await send("feed", null, "GET", k);
  waiting = false;
  if (key !== k) return;
  if (out.status === 410 || out.ended) { key = ""; return end("ended"); }
  if (out.ok) { if (out.target && out.target !== st.target) set({ target: out.target }); queue.push(...(out.items || []).filter((i) => !(hush && i.kind === "alert"))); }
  next();
}
function next() {
  if (!conv || !queue.length) return;
  const now = performance.now();
  if (st.mode === "speaking" || now - heardAt < QUIET_MS || now - spokeAt < QUIET_MS) return;
  const item = queue.shift();
  const text = `[board] ${item.text}`;
  try {
    conv.sendUserMessage(text);
    spokeAt = performance.now();
  } catch {}
  say("board", item.text);
}
export function mute(on = !st.muted) { if (!conv) return; try { conv.setMicMuted(on); } catch {} set({ muted: on }); }
// The call is over here: the board is told (unless it said so itself), and the microphone is let go.
function finish(note, why = "hung up") {
  const k = key;
  clearInterval(timer); timer = null; queue = [];
  try { lock?.release(); } catch {}
  lock = null; conv = null;
  if (k) send("end", { why, audio: hear.report() }, "POST", k);
  key = ""; hear.stop(); fresh();
  const secs = st.since ? Math.round((Date.now() - st.since) / 1000) : 0;
  set({ state: "idle", mode: "", muted: false, note, since: 0, endedAt: Date.now(), secs });
}
export async function end(why = "hung up") {
  const c = conv;
  if (st.state === "starting" && !c) { cancelled = true; return; }
  if (!c && !key) return;
  finish(why === "hung up" ? "You hung up" : "Call ended", why);
  try { await c?.endSession(); } catch {}
}
window.addEventListener("pagehide", () => { if (key) finish("", "the page closed"); });
