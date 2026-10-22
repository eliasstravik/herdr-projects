// One task, whole: its title, the buttons for what can be done to it now, and the task panel's parts (status, what
// it needs from you, description, reviews, evidence, what the agent said, settings, session). The
// parts and the buttons come from the board (webui/panel-model.js, webui/side-model.js); this file lays them out.
// Under it, All settings: every work setting of the task (or automation) with its plain label, what it means, its
// value and the place it comes from, each one changed right there (webui/settings-model.js).
import { h, icon, ink } from "./dom.js";

const cap = (s) => (s ? s[0].toUpperCase() + s.slice(1) : s);
// Buttons that end something are not the ones to hit by mistake.
const QUIET = ["stop", "archive", "back"];

function item(x, onGo) {
  if (x.k === "gap") return h("div.sw-gap");
  // A firing of an automation that made a task: the line goes to that task.
  if (x.k === "text" && x.go) return h("button.sw-text.sw-go", { type: "button", title: `Open ${x.go}`, onclick: () => onGo?.(x.go) }, ink(x.value));
  // A line the board set in under another (a finding's further lines under its first) stays set in when it wraps.
  if (x.k === "text") { const lead = /^ */.exec(x.value[0]?.t || "")[0].length; return h("p.sw-text", lead ? { style: `padding-left:${lead * 0.5}em` } : null, ink(lead ? [{ ...x.value[0], t: x.value[0].t.slice(lead) }, ...x.value.slice(1)] : x.value)); }
  return h("div.sw-fact", h("dt.sw-label", x.label), h("dd.sw-value",
    x.bar === undefined ? null : h("span.sw-bar.sw-bar-wide", { role: "progressbar", "aria-valuenow": x.bar, "aria-valuemin": 0, "aria-valuemax": 100 }, h("span.sw-bar-fill", { style: `width:${x.bar}%` })),
    x.lines.map((l) => h("span.sw-vline", ink(l)))));
}
export function actionsEl(snap, { onPress, onMenu }) {
  const buttons = (snap.actions || []).map((b, n) => h(`button.btn${n === 0 && !QUIET.includes(b.id) ? ".btn-primary" : ""}.sw-act`, { type: "button", onclick: () => onPress(b.id) }, cap(b.label)));
  // Into its agent: the open key, as the hint line has it where it says so (enter only folds and sends now).
  if (snap.canOpen && !(snap.actions || []).some((b) => b.id === "open")) buttons.push(h("button.btn.sw-act", { type: "button", onclick: () => onPress("open") }, "Open this task's agent"));
  buttons.push(h("button.btn.btn-ghost.sw-act", { type: "button", "aria-haspopup": "dialog", onclick: onMenu }, "More…"));
  return h("div.sw-actions", buttons);
}
export function panelEl(snap, handlers) {
  const p = snap.panel;
  if (!p) return null;
  return h("article.sw-panel", { "aria-label": p.automation ? "Automation" : "Task" },
    h("header.sw-panel-head",
      // One back arrow at a time: on a phone this one stands for the Board's own while a task is open (board.css); beside
      // the list on a wide screen, where the list never left, it is a close mark instead.
      h("button.icon-button.sw-panel-back", { type: "button", "aria-label": "Back to the list", title: "Back to the list", onclick: handlers.onClose }, h("span.sw-narrow", icon("back")), h("span.sw-wide", icon("close"))),
      h("h3.sw-panel-title", p.tag ? h("span.sw-id", p.tag + " ") : null, p.title || "(no title)")),
    actionsEl(snap, handlers),
    h("div.sw-parts", p.parts.map((part) => h("section.sw-part",
      h(`h4.sw-part-head${part.color ? `.sw-c-${part.color}` : ""}`, part.head),
      h("dl.sw-facts", part.items.map((x) => item(x, handlers.onGo)))))));
}

// ---- All settings ------------------------------------------------------------------------------------------------------
// Folded until asked for; it stays as you left it while the page is kept fresh.
let unfolded = false, shown = 0;
function setRow(x, whose, canSet, onSet) {
  const pick = x.words && h("select.sw-set-pick", { "aria-label": x.label, disabled: !canSet, onchange: (e) => onSet(x.name, e.target.value) },
    x.words.map((w) => h("option", { value: w.value, selected: w.now }, w.say)));
  // Typed: saved when you press enter or leave the box, and only when it says something new.
  const box = !x.words && h("input.sw-set-box", { type: "text", value: x.value, "aria-label": x.label, title: x.hint, placeholder: x.hint, disabled: !canSet, autocomplete: "off", spellcheck: false,
    onchange: (e) => { const text = e.target.value.trim(); if (text && text !== x.value) onSet(x.name, text); else e.target.value = x.value; } });
  return h(`div.sw-set${x.own ? ".is-own" : ""}`,
    h("div.sw-set-top", h("span.sw-set-label", x.label), pick || box),
    x.means ? h("p.sw-set-means", x.words ? `On: ${x.means}.` : `Means: ${x.means}.`) : null, // a typed value has no "on": what it is
    h("p.sw-set-from", [x.own ? `* ${whose} only` : "", x.from].filter(Boolean).join(" · "), h("span.sw-set-name", `${x.own || x.from ? " · " : ""}${x.name}`),
      x.own && canSet ? h("button.sw-set-back", { type: "button", title: "Back to what it would be without its own answer", onclick: () => onSet(x.name, null) }, "Put back") : null));
}
export function settingsEl(snap, { onSet }) {
  const s = snap.settings;
  if (!s || !s.rows.length) return null;
  if (s.show !== shown) { if (s.show > shown && shown >= 0) unfolded = true; shown = s.show; }
  const set = (name, value) => onSet(s.id, name, value);
  const el = h("details.sw-settings", { open: unfolded, ontoggle: (e) => { unfolded = e.target.open; } },
    h("summary.sw-settings-head", "All settings", h("span.sw-settings-note", `${s.rows.filter((x) => x.own).length} of ${s.whose}'s own`)),
    h("p.sw-set-means", s.canSet ? `Every setting. A change here is for ${s.whose} only, and marked *.` : "Read-only here."),
    s.rows.map((x) => setRow(x, s.whose, s.canSet, set)));
  return el;
}
