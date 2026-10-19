// What comes up over the board: the question it asks (approve a merge, remove a task), the
// More menu with every other action, the choice of whose tasks to show, and the board's short message to you. The question's words and answers and the
// menu's entries come from the board (webui/side-model.js): each button presses the key it stands for.
/* global document */
import { h, icon, get } from "./dom.js";

const cap = (s) => (s ? s[0].toUpperCase() + s.slice(1) : s);
const GROUPS = { ctrl: "Do", alt: "Show" };

// The one question. It stays until it is answered: a tap beside it is not an answer.
export function askEl(ask, onPress) {
  if (!ask) return null;
  return h("div.sw-scrim", h("div.sw-sheet", { role: "alertdialog", "aria-modal": "true", "aria-label": ask.q },
    h("p.sw-ask-q", ask.q),
    ask.detail ? h("p.sw-ask-detail", ask.detail) : null,
    h("div.sw-ask-keys", // A yes that cannot be undone (a merge, a remove) is never the filled button: neither answer is.
    ask.keys.map((k, n) => h(`button.btn${n === 0 && !ask.danger ? ".btn-primary" : ""}.sw-act`, { type: "button", onclick: () => onPress(k.id) }, cap(k.label))))));
}
export function menuEl(snap, { onPress, onClose, onFind }) {
  const title = snap.list.groups.flatMap((g) => g.rows).find((r) => r.id === snap.list.selected)?.title || "";
  const group = (name) => { const items = snap.menu.filter((i) => i.group === name); return items.length ? h("section.sw-menu-group", h("h4.sw-menu-name", GROUPS[name] || name), items.map((i) => h("button.sw-menu-item", { type: "button", onclick: () => { onClose(); onPress(i.id); } }, cap(i.label)))) : null; };
  return h("div.sw-scrim", { onclick: (e) => { if (e.target === e.currentTarget) onClose(); } }, h("div.sw-sheet.sw-menu", { role: "dialog", "aria-modal": "true", "aria-label": "More" },
    h("header.sw-menu-head", h("span.sw-menu-title", title ? `For "${title}"` : "More"), h("button.icon-button", { type: "button", "aria-label": "Close", onclick: onClose }, icon("close"))),
    group("ctrl"),
    h("section.sw-menu-group", h("h4.sw-menu-name", "Find"), h("button.sw-menu-item", { type: "button", onclick: () => { onClose(); onFind(); } }, "Find: show only the tasks that match")),
    group("alt")));
}
// Whose tasks to show: every board the sidebar's Boards section lists (webui/boards.rs), each with how it stands, the
// one shown now checked. A machine that does not answer says so, and can still be picked: its board says why.
export async function pickerEl(scope, { onPick, onClose }) {
  const out = await get("boards");
  const rows = out.ok ? out.boards || [] : [];
  return h("div.sw-scrim", { onclick: (e) => { if (e.target === e.currentTarget) onClose(); } }, h("div.sw-sheet.sw-menu", { role: "dialog", "aria-modal": "true", "aria-label": "Whose tasks to show" },
    h("header.sw-menu-head", h("span.sw-menu-title", "Whose tasks to show"), h("button.icon-button", { type: "button", "aria-label": "Close", onclick: onClose }, icon("close"))),
    rows.length ? rows.map((b) => h(`button.sw-menu-item.sw-pick-item${b.off ? ".is-off" : ""}`, { type: "button", "aria-current": b.scope === scope ? "true" : null, onclick: () => (b.scope === scope ? onClose() : onPick(b.scope)) },
      h("span.sw-pick-copy", h("span.sw-pick-name", b.name), h("span.sw-pick-line", b.line)), b.scope === scope ? icon("check") : null))
      : h("p.sw-set-means", out.msg || "The machines could not be listed just now.")));
}
// The board's message: a short line that goes by itself; one that says something went wrong stays until dismissed.
export function toast(host, msg, onDismiss) {
  host.replaceChildren();
  if (!msg) return;
  const bad = msg.kind === "red";
  const el = h(`div.sw-toast.sw-toast-${msg.kind}`, { role: bad ? "alert" : "status" }, h("span.sw-toast-text", msg.text),
    bad ? h("button.icon-button.sw-toast-x", { type: "button", "aria-label": "Dismiss", onclick: () => { el.remove(); onDismiss(); } }, icon("close")) : null);
  host.append(el);
  if (!bad) setTimeout(() => el.remove(), 6000);
}
// `theirs(e)`: an Escape that is the web UI's own (in its chat's box or its menus under a docked chat): left to it.
export const trapEscape = (fn, theirs = () => false) => { const on = (e) => { if (e.key === "Escape" && !theirs(e)) { e.stopPropagation(); fn(); } }; document.addEventListener("keydown", on, true); return () => document.removeEventListener("keydown", on, true); };
