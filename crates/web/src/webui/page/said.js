// The board's own conversation, where the web UI has no chat to show (dock.js): drawn as the web UI draws its own chat
// (ChatView.css there): your words as a card on the right, theirs as plain prose on the left, the board's and the
// reviewer's under their name, and a program's screen (one whose conversation cannot be read) as it is. It opens on its
// end and follows new words while you are there; scrolled up, it stays where you read, also when the board lays
// itself out again (full screen, a new answer from the board).
/* global requestAnimationFrame */
import { h } from "./dom.js";

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
// Who said it, as the terminal board's box says it (view/chat_text.rs): nobody's name over your own words or over the
// replies of the one you talk to (the bar names them); the reviewer's, autopilot's, the board's and a screen's over theirs.
const own = (p) => p.mine || p.by === "You" || !p.by || (!p.sent && !p.raw && !p.tone && p.by !== "Reviewer");
const partEl = (p) => h(`article.sw-turn-said${p.mine ? ".is-mine" : ""}${p.raw ? ".is-raw" : ""}${p.tone ? `.sw-c-${p.tone}` : ""}`,
  !own(p) ? h("div.sw-by", p.by) : null,
  h("div.sw-said-text", p.raw ? p.text : prose(p.text)));

export function saidEl() {
  let parts = [], key = "";
  const text = h("div.sw-said", { role: "log", "aria-label": "The conversation", tabIndex: 0 });
  const el = h("div.sw-said-box", text);
  const atEnd = () => text.scrollTop + text.clientHeight >= text.scrollHeight - 8;
  return {
    el,
    // The conversation as the board sends it (nothing said yet: the box's own words say what it does).
    set(list) {
      const now = JSON.stringify(list);
      if (now === key) return;
      const end = !key || atEnd();
      key = now; parts = list;
      text.replaceChildren(...parts.map(partEl));
      if (end) requestAnimationFrame(() => { text.scrollTop = text.scrollHeight; });
    },
    // Shown or hidden: only a change does anything (the board's layout asks on every update). Shown again, it opens
    // on its end, where the new words come.
    show(on) {
      if (el.hidden === !on) return;
      el.hidden = !on;
      if (on) requestAnimationFrame(() => { text.scrollTop = text.scrollHeight; });
    },
  };
}
