// Hearing the call: the voice service's client plays the voice agent through a hidden <audio> element it adds to the
// page (LiveKit's track.attach()), and never says when that playback fails. This watches those elements: it asks them
// to play, measures the sound that arrives on their track, and says when the page is not allowed to play sound
// (`blocked`: a tap fixes it, `unlock`). `webAudio` plays the same sound through the page's own audio context instead,
// a second way out of the browser for one where the element stays silent. What it saw is reported to the board
// (`report`), so `herdr-projects voice` can say why a call was silent.
/* global document, window, AudioContext, MutationObserver, MediaStream, navigator */
let ctx = null, observer = null, timer = null, via = "element";
const seen = new Map(); // element → { analyser, peak }
let st = { blocked: false, elements: 0, playing: 0, peak: 0, ctx: "", via };
const subs = new Set();
const set = (patch) => { st = { ...st, ...patch }; for (const f of subs) { try { f(st); } catch {} } };
export const watch = (f) => { subs.add(f); f(st); return () => subs.delete(f); };
export const state = () => st;

function context() { if (!ctx) { try { ctx = new AudioContext(); } catch { ctx = null; } } return ctx; }
async function adopt(el) {
  if (seen.has(el) || !(el.srcObject instanceof MediaStream)) return;
  const entry = { analyser: null, peak: 0, at: Date.now() };
  seen.set(el, entry);
  el.muted = via === "webaudio"; el.volume = 1;
  try { await el.play(); } catch (e) { if (e && e.name === "NotAllowedError") set({ blocked: true }); }
  const c = context();
  if (c && el.srcObject.getAudioTracks().length) {
    try { const src = c.createMediaStreamSource(new MediaStream(el.srcObject.getAudioTracks())); const an = c.createAnalyser(); an.fftSize = 512; src.connect(an); entry.analyser = an; entry.src = src; if (via === "webaudio") src.connect(c.destination); } catch {}
  }
}
function look() {
  for (const el of document.querySelectorAll("audio")) adopt(el);
  let playing = 0, stuck = 0, peak = st.peak;
  const buf = new Float32Array(512);
  for (const [el, e] of seen) {
    if (!el.isConnected) { seen.delete(el); continue; }
    if (!el.paused && el.readyState >= 2) playing++; else if (Date.now() - e.at > 1500) stuck++;
    if (e.analyser) { e.analyser.getFloatTimeDomainData(buf); for (const v of buf) peak = Math.max(peak, Math.abs(v)); }
  }
  const c = context();
  set({ elements: seen.size, playing, peak: Math.round(peak * 1000) / 1000, ctx: c ? c.state : "none", blocked: st.blocked || (c && c.state !== "running" && !navigator.userActivation?.hasBeenActive) || (stuck > 0 && playing === 0 && via === "element") });
}
// Start watching, at the start of a call. An audio context made now tells whether the page may play sound.
export async function start() {
  stop();
  const c = context();
  try { await c?.resume(); } catch {}
  observer = new MutationObserver(look);
  observer.observe(document.body, { childList: true, subtree: true });
  timer = window.setInterval(look, 500);
  look();
}
export function stop() {
  observer?.disconnect(); observer = null;
  if (timer) window.clearInterval(timer); timer = null;
  for (const [, e] of seen) { try { e.src?.disconnect(); } catch {} }
  seen.clear();
  set({ blocked: false, elements: 0, playing: 0, peak: 0 });
}
// A tap: the page may play sound from now on; every element plays again, and the context runs.
export async function unlock() {
  const c = context();
  try { await c?.resume(); } catch {}
  for (const [el] of seen) { el.muted = via === "webaudio"; el.volume = 1; try { await el.play(); } catch {} }
  set({ blocked: false });
  look();
}
// The second way out: the sound through the page's own audio context, the element silent (no echo of it twice).
export async function webAudio() {
  via = "webaudio";
  await unlock();
  const c = context();
  for (const [el, e] of seen) { el.muted = true; try { e.src?.connect(c.destination); } catch {} }
  set({ via });
}
// What was seen, for the board: no words of yours, only how the sound stood.
export function report() {
  return { elements: st.elements, playing: st.playing, peak: st.peak, ctx: st.ctx, blocked: st.blocked, via: st.via, activated: !!navigator.userActivation?.hasBeenActive, agent: navigator.userAgent.replace(/^Mozilla\/5\.0 /, "").slice(0, 160) };
}
