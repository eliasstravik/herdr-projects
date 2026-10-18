// The conversation above the box, as the terminal board's chat box holds it: the newest thing said, whoever said it,
// under a line that names who said it, and the rest of the conversation above it.
// It opens on the newest and holds only that; going up (the Earlier button, the wheel or a swipe at its top) opens it to
// the conversation and scrolls up through it, half a box at a time with the buttons; Later goes down, and once more from
// the end closes it to the newest again. A new message starts at its beginning; only a program's screen (one whose
// conversation cannot be read) at its end. Opened, it stays open while the same chat shows: on what you were reading,
// or on its end, where the new message comes, when you were there.
/* global requestAnimationFrame, ResizeObserver */
import { h, icon } from "./dom.js";

// Markdown read as the terminal board reads it (view/chat_text.rs), its marks left out: bold is bold here, a
// headline's marks and a code block's fences go, and a code block's lines keep the code's type.
function prose(text) {
  const out = [];
  let code = false;
  for (const line of text.split("\n")) {
    if (/^\s*(```|~~~)/.test(line)) { code = !code; continue; }
    if (out.length) out.push("\n");
    if (code) { out.push(h("code.sw-code", line)); continue; }
    line.replace(/^(\s*)#{1,6}\s+/, "$1").split(/\*\*([^*\n]+)\*\*/).forEach((bit, i) => { if (bit) out.push(i % 2 ? h("strong", bit) : bit); });
  }
  return out;
}
// One thing said: who said it, in words, then the words. A program's screen stays as it is.
const partEl = (p) => h(`div.sw-said-turn${p.mine ? ".is-mine" : ""}${p.raw ? ".is-raw" : ""}${p.tone ? `.sw-c-${p.tone}` : ""}`, p.by ? h("div.sw-by", `${p.by}:`) : null, h("div.sw-said-text", p.raw ? p.text : prose(p.text)));

export function saidEl() {
  // `start`: a new message is to be placed at its beginning, once it can be laid out (it may come while hidden).
  // `from`: the part it opens on (the newest thing said).
  // `full`: the conversation fills the board (its button, or Esc and the back arrow to leave), the whole of it open.
  let parts = [], from = 0, open = false, raw = false, swipe = null, start = false, full = false;
  const text = h("div.sw-said", { role: "log", "aria-label": "The conversation", tabIndex: 0 });
  const up = h("button.icon-button.sw-said-up", { type: "button", "aria-label": "Earlier messages", title: "Earlier messages" }, icon("up"));
  const down = h("button.icon-button.sw-said-down", { type: "button", "aria-label": "Later messages", title: "Later messages" }, icon("down"));
  const grow = h("button.icon-button.sw-said-full", { type: "button" });
  const el = h("div.sw-said-box", text, h("div.sw-said-nav", grow, up, down));
  const half = () => Math.max(24, Math.round(text.clientHeight / 2));
  const atEnd = () => text.scrollTop + text.clientHeight >= text.scrollHeight - 2;
  // The buttons show only where there is more that way (down from the end of an opened conversation closes it).
  // Hidden, nothing can be measured: it is measured again when it shows (and whenever its size changes).
  const mark = () => {
    if (el.hidden || !text.clientHeight) return;
    up.hidden = !(text.scrollTop > 1 || (!open && from > 0));
    down.hidden = !(!atEnd() || (open && !full));
    el.classList.toggle("is-open", open);
  };
  const draw = () => { text.replaceChildren(...(open ? parts : parts.slice(from)).map(partEl)); };
  // Open the conversation where what it opened on stands now, so nothing jumps; then `by` up.
  const opening = (by) => {
    const was = text.scrollTop;
    open = true; draw();
    const newest = text.children[from];
    text.scrollTop = (newest ? newest.offsetTop - text.firstElementChild.offsetTop : 0) + was - by;
    mark();
  };
  // Where you read in an opened conversation: the first thing said in view, and how far above the top it starts.
  const rel = (n) => n.offsetTop - text.firstElementChild.offsetTop;
  const reading = () => { const i = [...text.children].findIndex((n) => rel(n) + n.offsetHeight > text.scrollTop); return i < 0 ? null : { i, off: rel(text.children[i]) - text.scrollTop }; };
  // How a conversation moved from `was` to `now`: the shift that lines up the most of what was said (the smallest on a
  // tie), so that something said twice is found where it was read, not at its other place.
  const same = (a, b) => !!a && !!b && a.by === b.by && a.text === b.text;
  const shift = (was, now) => {
    let best = 0, most = -1;
    for (let d = -now.length; d <= was.length; d++) {
      let n = 0;
      for (let k = Math.max(0, -d); k < now.length && k + d < was.length; k++) if (same(now[k], was[k + d])) n++;
      if (n > most || (n === most && Math.abs(d) < Math.abs(best))) { most = n; best = d; }
    }
    return best;
  };
  const close = () => { open = false; draw(); text.scrollTop = raw ? text.scrollHeight : 0; mark(); };
  const scroll = (by) => { text.scrollBy({ top: by, behavior: "smooth" }); };
  up.addEventListener("click", () => { if (!open && from > 0 && text.scrollTop <= 1) opening(half()); else scroll(-half()); });
  down.addEventListener("click", () => { if (open && atEnd()) { if (!full) close(); } else scroll(half()); });
  // Full screen: on a phone the box over the chat is a few lines high. The board makes room (board.css: .sw-chat-full).
  const look = () => { grow.replaceChildren(icon(full ? "shrink" : "expand")); grow.title = full ? "Back to the board" : "Full screen"; grow.setAttribute("aria-label", grow.title); grow.setAttribute("aria-pressed", String(full)); };
  const setFull = (on) => {
    if (full === on) return;
    full = on; look();
    el.closest(".sw-board")?.classList.toggle("sw-chat-full", on);
    if (on && !open && from > 0) opening(0); else if (!on && open) close(); else mark();
  };
  look();
  grow.addEventListener("click", () => setFull(!full));
  // A new message starts at its beginning (a screen at its end), once it can be laid out.
  const begin = () => {
    const place = () => { if (start && !el.hidden && text.clientHeight) { text.scrollTop = raw ? text.scrollHeight : 0; start = false; } mark(); };
    place(); requestAnimationFrame(place);
  };
  text.addEventListener("scroll", mark, { passive: true });
  // Its size changes (shown again, the window, the phone's keyboard): what there is more of is measured again.
  if (typeof ResizeObserver === "function") new ResizeObserver(() => mark()).observe(text);
  // At the top of the newest message, going further up opens the conversation: the wheel, or a finger moving down.
  text.addEventListener("wheel", (e) => { if (e.deltaY < 0 && !open && from > 0 && text.scrollTop <= 1) { e.preventDefault(); opening(Math.min(half(), -e.deltaY * 2)); } }, { passive: false });
  text.addEventListener("touchstart", (e) => { swipe = e.touches[0]?.clientY ?? null; }, { passive: true });
  text.addEventListener("touchmove", (e) => { const y = e.touches[0]?.clientY; if (swipe == null || y == null) return; if (y - swipe > 12 && !open && from > 0 && text.scrollTop <= 1) { swipe = null; opening(24); } }, { passive: true });
  // The keys the terminal has for it, here while the conversation has the focus: page up and down.
  text.addEventListener("keydown", (e) => { if (e.key === "PageUp") { e.preventDefault(); up.click(); } else if (e.key === "PageDown") { e.preventDefault(); down.click(); } });
  return {
    el,
    // A new conversation or a new message (also in the same words: `fresh`) starts over; otherwise where you are stays.
    // In the `same` chat an opened conversation stays open: on what you read (found again as the oldest drop off), or
    // on its end when you were there.
    set(list, fresh, isRaw, at = list.length - 1, sameChat = false) {
      if (!fresh) return;
      const keep = sameChat && open && !el.hidden, end = keep && atEnd(), was = keep ? reading() : null, before = parts;
      parts = list; raw = isRaw; open = keep; from = Math.min(Math.max(0, at | 0), Math.max(0, list.length - 1));
      draw();
      if (!keep) { start = true; begin(); if (full) requestAnimationFrame(() => { if (full && !open && from > 0) opening(0); }); return; }
      const i = was ? Math.min(Math.max(0, was.i - shift(before, parts)), parts.length - 1) : -1;
      if (end) text.scrollTop = text.scrollHeight; else if (i >= 0) text.scrollTop = rel(text.children[i]) - was.off;
      mark();
    },
    // Shown or hidden (Hide and Show, or nothing to show): shown again, the buttons are measured again.
    show(on) {
      if (el.hidden === !on) return;
      el.hidden = !on;
      if (on) begin(); else setFull(false);
    },
    // Full screen, or not; with no argument: whether it is.
    full(on) { if (on === undefined) return full; setFull(!!on && !el.hidden); return full; },
  };
}
