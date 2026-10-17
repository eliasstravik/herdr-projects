// The board over the web UI's page: its frame (the header, the list, one task, the chat), asking the board for what
// to show, and sending it what you do. It is kept fresh while it is open: asked every two seconds, drawn again only
// where something changed. Nothing about a task is worked out here: every word, colour and button comes from the
// board (crates/web/src/webui/*-model.js), and a button only says which key of the board it presses.
/* global document, window, location, history, PopStateEvent, CustomEvent */
import { h, icon, get, post } from "./dom.js";
import { listEl } from "./list.js";
import { panelEl, settingsEl } from "./panel.js";
import { chatEl } from "./chat.js";
import { askEl, menuEl, toast, trapEscape } from "./sheets.js";

let root = null, timer = null, closing = null, snap = null, mark = "", busy = false;
let menuOpen = false, finding = false, seenMsg = -1, seenGo = null, chat = null, untrap = null, wentTo = 0;
const drawn = { list: "", panel: "", settings: "", ask: "", menu: "", chips: "" };
const part = (name) => root.querySelector(`.sw-${name}`);

export const isOpen = () => !!root;
// The sidebar's Boards section (boards.js) is told when the board opens, closes or shows another machine's board.
const tell = (detail = {}) => window.dispatchEvent(new CustomEvent("sw-board", { detail }));
// Beside the web UI's sidebar where it stands open on a wide screen; the whole screen on a phone.
function fit() {
  if (!root) return;
  // (The sidebar's own box, its right border included: its shell is a pixel narrower.)
  const side = document.querySelector(".sidebar") || document.querySelector(".sidebar-shell"), box = side?.getBoundingClientRect();
  const beside = box && box.width > 0 && box.left >= 0 && box.right < window.innerWidth - 400 && window.innerWidth > 768;
  root.style.left = beside ? `${Math.round(box.right)}px` : "0";
}
// Into a task's agent: the web UI's own view of its pane, on this machine or on another. Its address says where the app is, and it follows a change
// of address as it follows Back, so the page is not loaded again.
function goTo(go) {
  const q = new URLSearchParams(location.search);
  for (const k of ["machine", "file", "settings", "section", "new", "cwd", "name", "agent", "sidebar"]) q.delete(k);
  if (go.machine) q.set("machine", go.machine); // on another of the web UI's PCs: its own name for it
  q.set("ws", go.workspace); q.set("pane", go.pane); q.set("view", go.view);
  history.pushState(null, "", `${location.pathname}?${q}`);
  window.dispatchEvent(new PopStateEvent("popstate", { state: null }));
}
function note(text) { const n = part("note"); n.textContent = text || ""; n.hidden = !text; }
function chips() {
  const f = snap.flags, head = snap.list.head, out = [];
  const chip = (text, title, fn) => h("button.sw-chip", { type: "button", title, onclick: fn }, text, h("span.sw-chip-x", { "aria-hidden": "true" }, "×"));
  if (f.archive) out.push(chip("Archive", "Back to the board", () => press("archiveView")));
  if (f.filter) out.push(chip(`Find: ${f.filter}`, "Clear the filter", () => call("find", { text: "" })));
  if (head.machines) out.push(chip(`All machines: ${head.machines}`, "Change whose board you see", () => press("scope")));
  if (head.repoScope) out.push(h("span.sw-chip", head.repoScope));
  if (head.only) out.push(chip(`Only ${head.only}`, "Change whose board you see", () => press("scope")));
  if (f.byRepo) out.push(chip("By repo", "Group by status", () => press("byRepo")));
  return out;
}
// Draw what changed. Each part is compared as text, so a tick that changed nothing touches nothing on the page.
function draw() {
  if (!root || !snap) return;
  root.classList.toggle("has-task", !!snap.panel);
  // The repo with its key beside it, dimmed: `gtm-agent · AGT`.
  const head = snap.list.head, where = part("where");
  where.replaceChildren(...[head.where, head.where && head.whereKey ? h("span.sw-repo-key", ` · ${head.whereKey}`) : null, head.session ? `${head.where ? " · " : ""}session ${head.session}` : null].filter(Boolean));
  const once = (key, value, fn) => { const text = JSON.stringify(value); if (drawn[key] !== text) { drawn[key] = text; fn(); } };
  once("chips", [snap.flags, snap.list.head], () => part("chips").replaceChildren(...chips()));
  once("list", snap.list, () => part("list-host").replaceChildren(listEl(snap.list, { onPick: (id) => call("pick", { id, open: true }), onMore: () => press("allDone"), onFold: (id) => call("fold", { id }) })));
  once("panel", [snap.panel, snap.actions, snap.canOpen, snap.role], () => { const el = panelEl(snap, { onPress: press, onMenu: () => { menuOpen = true; draw(); }, onClose: () => call("pick", { id: snap.list.selected, open: false }), onGo: (id) => call("pick", { id, open: true }) }); const host = part("task-host"), top = host.scrollTop; part("panel-host").replaceChildren(...(el ? [el] : [])); host.scrollTop = top; });
  // All settings stands under the task in a place of its own, drawn again only when a setting changed: what you
  // are typing into one of its boxes is not taken away by the task's status moving on.
  once("settings", snap.settings, () => { const host = part("task-host"), top = host.scrollTop, went = snap.settings?.show || 0, el = settingsEl(snap, { onSet: (id, name, value) => call("set", { id, name, value }) }); part("settings-host").replaceChildren(...(el ? [el] : [])); host.scrollTop = top; if (el && went > wentTo) el.scrollIntoView({ block: "start" }); wentTo = went; });
  once("ask", snap.ask, () => part("ask-host").replaceChildren(...[askEl(snap.ask, press)].filter(Boolean)));
  once("menu", menuOpen ? [snap.menu, snap.list.selected] : null, () => part("menu-host").replaceChildren(...(menuOpen ? [menuEl(snap, { onPress: press, onClose: () => { menuOpen = false; draw(); }, onFind: () => { finding = true; draw(); part("find").focus(); } })] : [])));
  part("find-row").hidden = !finding;
  chat.update(snap);
  if (snap.msg && snap.msg.n !== seenMsg) { seenMsg = snap.msg.n; toast(part("toast-host"), snap.msg, () => press("back")); }
  if (snap.go && seenGo !== null && snap.go.n !== seenGo) { seenGo = snap.go.n; return goTo(snap.go); }
  seenGo = snap.go ? snap.go.n : 0;
}
function take(out) {
  if (!root) return false;
  if (!out.ok) { note(out.status === 401 ? "Log in to the herdr web UI to see the board." : out.msg || "The board is not answering."); return false; }
  note("");
  if (out.same) return true;
  mark = out.mark; snap = out;
  draw();
  tell({ scope: snap.flags.scope });
  return true;
}
async function refresh() { if (!root || busy) return; take(await get("board", { have: mark })); }
// Asked again at once, whole: another machine's board was picked in the sidebar while the board is open.
export function freshen() { if (root) { mark = ""; refresh(); } }
// Something you did: sent, and drawn from the answer at once.
async function call(what, body) {
  busy = true;
  const out = await post(what, body);
  busy = false;
  if (!out.ok && out.status && out.status !== 401) { toast(part("toast-host"), { text: out.msg || "That did not work.", kind: "red" }, () => {}); return out; }
  take(out);
  return out;
}
// A key that brings words back (a draft that was put away, words taken back to change) puts them in the text box.
const press = async (id) => { const out = await call("press", { id }); if (out.draft) chat?.fill(out.draft); return out; };

export function openBoard({ onClose }) {
  if (root) return;
  closing = onClose; snap = null; mark = ""; seenGo = null; seenMsg = -1; menuOpen = false; finding = false;
  for (const k of Object.keys(drawn)) drawn[k] = "";
  chat = chatEl({ onSend: async (text, delegate, to) => { const out = await call("send", { text, delegate, to }); return !out.ok || !!out.kept; }, onPress: press });
  let wait = null;
  const find = h("input.sw-find", { type: "search", placeholder: "Type to filter, archived tasks too", "aria-label": "Find tasks", oninput: (e) => { clearTimeout(wait); const text = e.target.value; wait = setTimeout(() => call("find", { text }), 200); } });
  root = h("section.sw-board", { role: "region", "aria-label": "Work board" },
    h("header.sw-head",
      h("button.icon-button.sw-back", { type: "button", "aria-label": "Close the board", title: "Close the board", onclick: () => closing?.() }, icon("back")),
      h("h2.sw-name", "Board"), h("span.sw-where"),
      h("span.sw-head-space"),
      h("button.icon-button", { type: "button", "aria-label": "Find tasks", title: "Find tasks", onclick: () => { finding = !finding; draw(); if (finding) find.focus(); else if (snap?.flags.filter) { find.value = ""; call("find", { text: "" }); } } }, icon("find")),
      h("button.icon-button", { type: "button", "aria-label": "More", title: "More", "aria-haspopup": "dialog", onclick: () => { menuOpen = true; draw(); } }, icon("more"))),
    h("div.sw-find-row", { hidden: true }, find),
    h("div.sw-chips"),
    h("p.sw-note", { hidden: true, role: "status" }),
    h("div.sw-body", h("div.sw-list-host"), h("div.sw-task-host", h("div.sw-panel-host"), h("div.sw-settings-host"))),
    h("div.sw-toast-host", { "aria-live": "polite" }),
    chat.el,
    h("div.sw-ask-host"), h("div.sw-menu-host"));
  document.body.append(root);
  // On a phone the Board entry is in the web UI's drawer: the drawer closes as it does when you pick a pane there.
  try { document.querySelector(".scrim.is-open")?.click(); } catch {}
  // Escape goes one step back, as it does on the terminal board: the menu, the open task, the chat's latest message
  // (hidden as its Hide button hides it, while nothing is typed or searched), then the board itself.
  untrap = trapEscape(() => {
    if (snap?.ask) return press("back"); if (menuOpen) { menuOpen = false; return draw(); } if (snap?.panel) return call("pick", { id: snap.list.selected, open: false });
    if (snap?.chat?.open && snap.chat.text && !finding && !chat.typing()) return press("chatMessage");
    closing?.();
  });
  fit();
  window.addEventListener("resize", fit);
  refresh();
  tell();
  timer = setInterval(() => { fit(); if (!document.hidden) refresh(); }, 2000);
}
export function closeBoard() {
  if (!root) return;
  clearInterval(timer); timer = null;
  untrap?.(); untrap = null;
  window.removeEventListener("resize", fit);
  chat?.destroy(); chat = null;
  root.remove(); root = null;
  tell();
}
