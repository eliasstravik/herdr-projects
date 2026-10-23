// Dictation in the box you type in (compose.js), as the herdr web UI dictates into its own chat input (VoiceInput.tsx
// and lib/voice.ts there, rebuilt here from what they do and none of their code): the mic in the lower left, tapped to
// start and tapped again to finish, or held and let go (or ctrl/cmd+shift+space held, as there); while it listens a
// pill over the box with Cancel, where it stands, the words heard so far, the level, the time and Done; Escape cancels.
// The words go in at the caret, never glued to a word, without taking the focus; where the web UI's server writes a
// tidied version afterwards, it takes their place unless you changed them meanwhile. The web UI's own engines and
// settings: its server (`/api/voice`, which the page reaches as the web UI's own page does, through the front door)
// when it has a key, else the browser's speech recognition; its Settings → Voice input (shown on auto only where it
// can work, and not on a phone), its language and its tidying. Its limits: two minutes, 10 MB.
/* global window, document, navigator, fetch, FormData, Blob, MediaRecorder, AudioContext, performance, requestAnimationFrame, cancelAnimationFrame, TextDecoder */
import { h, icon, webUiSettings } from "./dom.js";

const HOLD_MS = 300, MAX_SECONDS = 120, MAX_BYTES = 10 * 1024 * 1024, SILENT_RMS = 0.003, SILENT_AFTER = 3000, BARS = 7, LONGEST = 20000;
const TYPES = [["audio/webm;codecs=opus", "webm"], ["audio/mp4", "m4a"], ["audio/webm", "webm"], ["audio/ogg;codecs=opus", "ogg"]];
// The web UI's own words for what went wrong.
const NOTES = { insecure: "Voice input needs HTTPS", permission: "Microphone permission was denied", no_mic: "No microphone found", not_configured: "Add an OpenAI API key in Settings > Voice input",
  provider_auth: "The OpenAI key was refused", provider: "Transcription failed", network: "Transcription failed", too_large: "Recording is too long", no_speech: "No speech was heard", unsupported: "This browser cannot record audio" };
// Its Settings → Voice input: on, off, or auto (the default): on a computer, where dictation can work.
export const wants = (s, phone) => { const v = ["on", "off", "auto"].includes(s.voiceInput) ? s.voiceInput : "auto"; return v === "on" || (v === "auto" && !phone); };
const locale = (s) => (typeof s.voiceLanguage === "string" && s.voiceLanguage !== "auto" ? s.voiceLanguage : navigator.languages?.[0] || navigator.language || "en-US");
const mod = (e) => (/Mac|iPhone|iPad/.test(navigator.platform || "") ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey);
export const isShortcut = (e) => mod(e) && e.shiftKey && !e.altKey && (e.code === "Space" || e.key === " ");
// The words in place of the selection, with one space against a word on either side; where they stand, for the tidied version.
export function insertAt(value, from, to, text) {
  const spoken = text.trim(), before = value.slice(0, from), after = value.slice(to);
  if (!spoken) return null;
  const lead = before && !/\s$/.test(before) ? " " : "", trail = after && !/^\s/.test(after) ? " " : "";
  const start = before.length + lead.length;
  return { value: before + lead + spoken + trail + after, start, end: start + spoken.length };
}
let asked = null;
const voiceStatus = () => (asked ||= fetch("/api/voice", { cache: "no-store", credentials: "same-origin" }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((s) => { if (!s) asked = null; return s; }));
async function* lines(body) {
  const reader = body.getReader(), dec = new TextDecoder();
  let buf = "";
  for (;;) {
    const { done, value } = await reader.read();
    buf += done ? dec.decode() : dec.decode(value, { stream: true });
    const all = buf.split("\n"); buf = done ? "" : all.pop() || "";
    for (const l of all) if (l.trim()) yield JSON.parse(l);
    if (done) return;
  }
}

// `write(value, caret)`: the box's words changed. `say(text)`: the note over the box. `keywords()`: words it may hear.
export function voiceEl({ box, write, say, keywords = () => [] }) {
  let dead = false, state = "idle", take = null, takes = 0, pressedAt = null, engine = null, frame = 0, spans = new Map(), waiting = [];
  const Speech = window.SpeechRecognition || window.webkitSpeechRecognition || null;
  const mime = typeof MediaRecorder !== "undefined" && navigator.mediaDevices?.getUserMedia ? TYPES.find(([t]) => MediaRecorder.isTypeSupported(t)) || null : null;
  const mic = h("button.sw-mic", { type: "button", "aria-label": "Dictate (hold)", title: "Dictate (hold)", "aria-pressed": "false" }, icon("mic"));
  const wrap = h("span.sw-voice", { hidden: true, data: { state } }, h("span.sw-voice-ring", { "aria-hidden": "true" }), mic);
  const label = h("span.sw-voice-label", { role: "status", "aria-live": "polite" }), partial = h("span.sw-voice-partial"), timer = h("span.sw-voice-timer", { "aria-hidden": "true" });
  const bars = h("span.sw-voice-bars", { "aria-hidden": "true" }, ...Array.from({ length: BARS }, () => h("span")));
  const cancelBtn = h("button.sw-voice-button", { type: "button", "aria-label": "Cancel dictation", title: "Cancel dictation", tabIndex: -1, onpointerdown: (e) => e.preventDefault(), onclick: () => cancel() }, icon("close"));
  const doneBtn = h("button.sw-voice-button.sw-voice-done", { type: "button", "aria-label": "Insert dictation", title: "Insert dictation", tabIndex: -1, onpointerdown: (e) => e.preventDefault(), onclick: () => finish() }, icon("check"));
  const pill = h("div.sw-voice-pill", { "aria-hidden": "true", data: { state } }, cancelBtn, h("span.sw-voice-dot", { "aria-hidden": "true" }), label, partial, bars, timer, doneBtn);
  const shown = () => !wrap.hidden;
  const set = (next) => {
    state = next; wrap.dataset.state = pill.dataset.state = next;
    const on = next !== "idle", rec = next === "starting" || next === "recording";
    pill.classList.toggle("is-open", on); pill.setAttribute("aria-hidden", String(!on)); cancelBtn.tabIndex = doneBtn.tabIndex = on ? 0 : -1;
    doneBtn.disabled = next === "transcribing";
    mic.setAttribute("aria-pressed", String(rec)); mic.replaceChildren(icon(rec ? "stop" : "mic"));
    mic.title = rec ? "Insert dictation" : "Dictate (hold)"; mic.setAttribute("aria-label", mic.title);
    label.textContent = next === "transcribing" ? "Transcribing…" : next === "starting" ? "Ready…" : next === "recording" ? "Recording" : "";
    if (!on) { partial.textContent = ""; timer.textContent = ""; }
  };
  const fail = (why) => say(NOTES[why] || NOTES.provider);
  // What was heard, at the caret; a tidied version only over its own words, while they are still as they went in.
  const apply = (r) => {
    if (dead) return;
    if (box.dataset.composing) { waiting.push(r); return; }
    const v = box.value, live = document.activeElement === box, [a, b] = live ? [box.selectionStart, box.selectionEnd] : [v.length, v.length];
    if (r.phase === "raw") {
      const n = insertAt(v, a, b, r.text);
      if (!n) return;
      if (n.value.length > LONGEST) return say("The dictation does not fit in the box");
      spans.set(r.take, { start: n.start, end: n.end, text: r.text.trim() });
      return write(n.value, n.end);
    }
    const s = spans.get(r.take); spans.delete(r.take);
    if (!s || a !== b || v.slice(s.start, s.end) !== s.text || r.text === s.text) return;
    const n = v.slice(0, s.start) + r.text + v.slice(s.end);
    if (n.length <= LONGEST) write(n, a >= s.end ? a + n.length - v.length : a);
  };
  box.addEventListener("compositionend", () => setTimeout(() => { for (const r of waiting.splice(0)) apply(r); }, 0));
  box.addEventListener("input", () => spans.clear()); // typed over: a late tidy-up would land on other words
  // The level: seven bars, and "No microphone input" after three silent seconds.
  const loop = () => {
    frame = requestAnimationFrame(loop);
    const t = take; if (!t) return;
    let rms = 0;
    if (t.analyser) { t.analyser.getFloatTimeDomainData(t.samples); let sum = 0; for (const x of t.samples) sum += x * x; rms = Math.sqrt(sum / t.samples.length); }
    const now = performance.now(), level = Math.min(1, Math.sqrt(rms) * 3);
    [...bars.children].forEach((b, i) => { b.style.transform = `scaleY(${Math.max(0.12, level * [0.45, 0.65, 0.85, 1, 0.85, 0.65, 0.45][i]).toFixed(3)})`; });
    if (state !== "recording" || t.at == null) return;
    if (rms >= SILENT_RMS || !t.analyser) t.loud = now;
    const sec = Math.floor((now - t.at) / 1000), left = MAX_SECONDS - sec;
    timer.textContent = `${Math.floor((left <= 10 ? Math.max(0, left) : sec) / 60)}:${String((left <= 10 ? Math.max(0, left) : sec) % 60).padStart(2, "0")}`;
    timer.classList.toggle("is-ending", left <= 10);
    label.textContent = now - t.loud >= SILENT_AFTER ? "No microphone input" : "Recording";
    if (sec >= MAX_SECONDS) finish();
  };
  const stopLoop = () => { cancelAnimationFrame(frame); frame = 0; for (const b of bars.children) b.style.transform = ""; };
  const close = (t) => { t.stream?.getTracks().forEach((k) => k.stop()); t.context?.close().catch(() => {}); };
  const settle = (t) => { if (take === t) take = null; if (!take) { stopLoop(); set("idle"); } };
  async function startServer(t) {
    try {
      t.stream = await navigator.mediaDevices.getUserMedia({ audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true } });
      if (take !== t) return close(t);
      if (t.context) { const src = t.context.createMediaStreamSource(t.stream); t.analyser = t.context.createAnalyser(); t.analyser.fftSize = 1024; t.samples = new Float32Array(1024); src.connect(t.analyser); }
      t.recorder = new MediaRecorder(t.stream, { mimeType: mime[0] });
      t.recorder.ondataavailable = (e) => { if (e.data.size) t.chunks.push(e.data); };
      t.recorder.onstart = () => { if (take === t) { t.at = t.loud = performance.now(); set("recording"); } };
      t.recorder.start(1000);
    } catch (e) { if (take !== t) return; close(t); take = null; stopLoop(); set("idle"); fail(e?.name === "NotAllowedError" || e?.name === "SecurityError" ? "permission" : "no_mic"); }
  }
  async function transcribe(t, blob) {
    const s = webUiSettings(), form = new FormData(), lang = locale(s).split("-")[0].toLowerCase();
    form.append("audio", blob, `voice.${mime[1]}`); form.append("mode", "chat"); form.append("polish", s.voicePolishChat === false ? "0" : "1");
    form.append("keywords", JSON.stringify([...new Set(keywords().map((k) => k.trim()).filter((k) => k && k.length <= 80))].slice(0, 50)));
    form.append("language", /^[a-z]{2}$/.test(lang) ? lang : "en");
    try {
      const r = await fetch("/api/voice/transcribe", { method: "POST", body: form, credentials: "same-origin", signal: t.abort.signal });
      if (!r.ok || !r.body) { const code = (await r.json().catch(() => null))?.error?.code; if (code === "voice_not_configured") asked = null; return fail({ voice_not_configured: "not_configured", audio_too_large: "too_large", provider_auth: "provider_auth" }[code] || "provider"); }
      let heard = "";
      for await (const ev of lines(r.body)) {
        if (t.abort.signal.aborted) return;
        if (ev.type === "delta") { heard += ev.text; partial.textContent = heard.replace(/\s+/g, " ").trim(); }
        else if (ev.type === "error") fail({ audio_too_large: "too_large", provider_auth: "provider_auth" }[ev.code] || "provider");
        else if (ev.text?.trim()) apply({ text: ev.text.trim(), phase: ev.type === "done" ? "raw" : "polished", take: t.id });
      }
    } catch (e) { if (!t.abort.signal.aborted) fail(e instanceof TypeError ? "network" : "provider"); }
  }
  function startBrowser(t) {
    const r = new Speech(); t.recognition = r;
    r.lang = locale(webUiSettings()); r.continuous = true; r.interimResults = true;
    r.onaudiostart = () => { if (take === t) { t.at = t.loud = performance.now(); set("recording"); } };
    r.onresult = (e) => { let fin = "", mid = ""; for (const x of e.results) { if (x.isFinal) fin += x[0].transcript; else mid += x[0].transcript; } t.final = fin; partial.textContent = (fin + mid).replace(/\s+/g, " ").trim(); };
    r.onerror = (e) => { if (!["no-speech", "aborted"].includes(e.error) && !t.discard) fail({ "not-allowed": "permission", "service-not-allowed": "permission", "audio-capture": "no_mic", network: "network" }[e.error] || "provider"); };
    r.onend = () => { if (!t.discard && t.final.trim()) apply({ text: t.final.replace(/\s+/g, " ").trim(), phase: "raw", take: t.id }); settle(t); };
    try { r.start(); } catch { take = null; stopLoop(); set("idle"); fail("provider"); }
  }
  function press() {
    if (state === "starting" || state === "recording") return finish();
    if (dead || state === "transcribing" || !engine || box.disabled) return;
    say(null);
    if (!window.isSecureContext) return fail("insecure");
    const t = { id: ++takes, engine, chunks: [], final: "", at: null, loud: 0, abort: new AbortController(), discard: false };
    take = t; pressedAt = performance.now();
    set("starting"); if (!frame) loop();
    if (engine === "server") { try { t.context = new AudioContext(); t.context.resume().catch(() => {}); } catch { t.context = null; } startServer(t); } else startBrowser(t);
  }
  function finish() {
    pressedAt = null;
    const t = take; if (!t) return;
    if (state === "starting") { t.discard = true; t.recognition?.abort(); close(t); return settle(t); }
    set("transcribing");
    if (t.engine === "browser") return t.recognition.stop();
    t.recorder.onstop = async () => {
      close(t);
      const blob = new Blob(t.chunks, { type: t.recorder.mimeType || mime[0] });
      if (!blob.size) fail("no_speech"); else if (blob.size > MAX_BYTES) fail("too_large"); else await transcribe(t, blob);
      settle(t);
    };
    if (t.recorder.state !== "inactive") t.recorder.stop(); else t.recorder.onstop();
  }
  function cancel() {
    pressedAt = null;
    const t = take; take = null;
    if (t) { t.discard = true; t.abort.abort(); if (t.recorder) { t.recorder.onstop = null; if (t.recorder.state !== "inactive") t.recorder.stop(); } t.recognition?.abort(); close(t); }
    waiting = []; stopLoop(); set("idle");
  }
  const release = () => { const at = pressedAt; pressedAt = null; if (at != null && performance.now() - at >= HOLD_MS) finish(); };
  // The mic acts on the press, so it can be held; the click a release makes after it is not a second press.
  let upAt = -Infinity;
  mic.addEventListener("pointerdown", (e) => { if (e.button !== 0) return; e.preventDefault(); mic.setPointerCapture?.(e.pointerId); press(); });
  mic.addEventListener("pointerup", () => { upAt = performance.now(); release(); });
  mic.addEventListener("pointercancel", release);
  mic.addEventListener("click", () => { if (performance.now() - upAt > 500) press(); });
  // Every listener on the page itself is let go with the box (`destroy`): a closed board dictates nothing.
  const offs = [];
  const on = (target, name, fn, opt) => { target.addEventListener(name, fn, opt); offs.push(() => target.removeEventListener(name, fn, opt)); };
  let held = false;
  on(window, "keydown", (e) => {
    if (state !== "idle" && e.key === "Escape") { e.preventDefault(); e.stopImmediatePropagation(); return cancel(); }
    if (!isShortcut(e) || !shown() || !engine || !box.isConnected) return;
    e.preventDefault(); e.stopPropagation();
    if (!e.repeat && !held) { held = true; press(); }
  }, true);
  on(window, "keyup", (e) => { if (held && [" ", "Shift", "Meta", "Control"].includes(e.key)) { held = false; release(); } }, true);
  on(window, "blur", () => { if (held) { held = false; release(); } });
  on(document, "visibilitychange", () => { if (document.hidden) { if (state === "recording") finish(); else if (state === "starting") cancel(); } });
  // Which engine, once the web UI has said whether its server has a key; shown only where the web UI would show it.
  const refresh = async () => {
    if (dead) return;
    const s = webUiSettings(), want = wants(s, window.matchMedia?.("(max-width: 768px)").matches);
    const st = want ? await voiceStatus() : null;
    engine = !want || !window.isSecureContext ? null : st?.configured && mime ? "server" : Speech ? "browser" : null;
    wrap.hidden = !want || (s.voiceInput !== "on" && !engine);
    mic.disabled = !engine || box.disabled;
    if (!engine && !wrap.hidden) mic.title = NOTES[!window.isSecureContext ? "insecure" : st?.configured ? "unsupported" : "not_configured"];
    if (wrap.hidden && state !== "idle") cancel();
  };
  refresh();
  on(window, "herdr:voice-config", () => { asked = null; refresh(); });
  on(window, "focus", refresh);
  // `sync()`: the box was turned off or on (a device that may only watch dictates nothing). `forget()`: the words were
  // sent as they are, so a tidied version that comes later has nowhere to go.
  return { el: wrap, pill, refresh, cancel, sync: () => { mic.disabled = !engine || box.disabled; if (box.disabled && state !== "idle") cancel(); }, forget: () => spans.clear(),
    // The board closed: what is being heard or written out is dropped, the microphone is let go, and nothing more is
    // put in the box (its words are kept with the page, and a new board has a box of its own).
    destroy: () => { cancel(); dead = true; for (const off of offs.splice(0)) off(); } };
}
