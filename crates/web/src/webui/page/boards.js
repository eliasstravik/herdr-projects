// The Boards section of the web UI's sidebar: under a heading that looks and folds as the web UI's own Agents heading
// does, one entry for all machines at the top (once there is another machine), then one per machine that has a board,
// each with a dot in the board's status colours and a short line of how it stands (`1 working · 2 to do`). Its rows are
// built of the web UI's own sidebar classes (AgentSidebar.tsx there), so they look as its agent rows do. Picking one
// shows that machine's board, as the board's own Other machines does: the section and the board are one browser's
// board, so the two stay in step.
// Every word comes from the board (webui/boards.rs).
/* global document, window */
import { h, icon, get, post } from "./dom.js";
import { isOpen } from "./board.js";

const EVERY = 4000;
// Folded as Agents is when it starts: on a phone (the sidebar is a drawer there), open on a wide screen. A fold stays
// as long as the page does, also when the web UI draws its sidebar again.
let folded = (() => { try { return window.matchMedia("(max-width: 768px)").matches; } catch { return false; } })();
let section = null, boards = null, scope = "", picking = false, drawn = "";
const svg = (name, cls) => { const s = icon(name); s.setAttribute("class", cls); return s; };

// A board's dot, in the colour of the first of its tasks that asks for a look; none while it is asked or away.
function dotEl(b) {
  const word = b.off ? "not reachable" : !b.dot ? "being asked" : b.dot === "quiet" ? "all quiet" : b.line;
  return h(`span.sw-bdot${b.dot ? `.sw-c-${b.dot}` : ""}${b.off ? ".is-off" : ""}`, { role: "img", "aria-label": `${b.name}: ${word}` });
}
function rowEl(b, onPick) {
  const selected = isOpen() && b.scope === scope;
  return h(`li.agent-item${selected ? ".is-selected" : ""}${b.off ? ".is-offline" : ""}`, { data: { swScope: b.scope } },
    h("button.agent-select.agent-row", { type: "button", title: b.title, "aria-current": selected ? "true" : null, onclick: () => pick(b.scope, onPick) },
      h("span.sidebar-mark", { "aria-hidden": "true" }, icon(b.scope === "*" ? "all" : "board")),
      h("span.agent-copy", h("span.agent-title", b.name), h("span.agent-context", b.line)),
      h("span.agent-row-status", dotEl(b))));
}
// Drawn again only when something changed, so a row under the pointer or the keyboard stays the same row.
function draw(onPick) {
  if (!section) return;
  const now = JSON.stringify([folded, boards, scope, isOpen()]);
  if (now === drawn) return;
  drawn = now;
  section.classList.toggle("is-collapsed", folded);
  const toggle = section.querySelector(".agent-section-toggle"), list = section.querySelector(".agent-list-contents");
  toggle.setAttribute("aria-expanded", folded ? "false" : "true");
  // Folded, the heading still says how things stand on every board: the dot of all of them together.
  const all = boards && (boards.find((b) => b.scope === "*") || boards[0]);
  toggle.replaceChildren(...[svg(folded ? "right" : "down", "agent-section-caret"), h("span", "Boards"), folded && all ? h("span.agent-section-count", dotEl(all)) : null].filter(Boolean));
  list.hidden = folded;
  list.replaceChildren(boards ? h("ul.agent-list", boards.map((b) => rowEl(b, onPick))) : h("p.agent-empty", { role: "status" }, "Reading the boards…"));
}
async function pick(to, onPick) {
  if (picking) return;
  picking = true;
  const out = await post("scope", { to });
  picking = false;
  if (out.ok) scope = to;
  draw(onPick);
  onPick(out);
}
async function look(onPick) {
  if (document.hidden) return;
  const out = await get("boards");
  if (!out.ok || !Array.isArray(out.boards)) return;
  boards = out.boards; scope = out.scope ?? "";
  draw(onPick);
}
// The section, made once: the web UI may draw its sidebar again (on a phone, each time its drawer opens), and the
// same section is put back in its place.
export function boardsEl(onPick) {
  if (section) return section;
  section = h("section.sw-boards", { "aria-label": "Boards", "data-sw-entry": true },
    h("button.agent-section-toggle.sidebar-section-label", { type: "button", "aria-controls": "sw-boards-list", onclick: () => { folded = !folded; draw(onPick); } }),
    h("div.agent-list-contents", { id: "sw-boards-list" }));
  draw(onPick);
  look(onPick);
  setInterval(() => look(onPick), EVERY);
  document.addEventListener("visibilitychange", () => look(onPick));
  // The board says when it opens, closes or shows another machine's board: the highlight follows at once.
  window.addEventListener("sw-board", (e) => { if (e.detail?.scope !== undefined) scope = e.detail.scope; draw(onPick); });
  return section;
}
