// The task list as the page draws it: the groups under their headings and one row per task. Everything shown comes
// from the board (webui/list-model.js): this file only lays it out. A tap on a row selects it, as the cursor does on
// the terminal board; the selected row has an Open button, which shows it whole.
/* global ResizeObserver, localStorage */
import { h, icon, ink, said, markEl } from "./dom.js";

// A task an automation made has its `[auto]` tag in the automations' colour.
const titleOf = (r) => { const at = r.made ? r.title.lastIndexOf("[auto]") : -1; return at < 0 ? r.title : [r.title.slice(0, at), h("span.sw-c-auto", "[auto]"), r.title.slice(at + 6)]; };

export function rowEl(r, { selected, opened, onPick, onOpen }) {
  const tags = h("span.sw-tags", r.review?.length ? ink(r.review, "sw-tag") : null,
    r.pr?.length ? (r.prUrl ? h("a.sw-tag", { href: r.prUrl, target: "_blank", rel: "noopener noreferrer", onclick: (e) => e.stopPropagation() }, ink(r.pr)) : ink(r.pr, "sw-tag")) : null);
  // A task that waits for a free slot has its place in the queue (Q#2) where a task at work has its percent.
  const meter = r.progress === null && !r.eta && !r.percent ? null : h("span.sw-meter", { className: `sw-meter sw-c-${r.color}` },
    r.progress === null ? null : h("span.sw-bar", { role: "progressbar", "aria-valuenow": r.progress, "aria-valuemin": 0, "aria-valuemax": 100 }, h("span.sw-bar-fill", { style: `width:${r.progress}%` })),
    r.percent ? h("span.sw-num", r.percent) : null, r.eta ? h("span.sw-num", r.eta) : null);
  const says = said(r.says);
  // Enter on the selected row opens it, as its Open button does; on any other row it selects, as a tap does.
  return h("li.sw-row", { className: `sw-row${selected ? " is-selected" : ""}${r.quiet ? " is-quiet" : ""}${r.archived ? " is-archived" : ""}${r.automation ? " is-automation" : ""}`, data: { id: r.id }, role: "option", "aria-selected": selected ? "true" : "false", tabIndex: 0,
    onclick: () => { if (!selected) onPick(r.id); }, onkeydown: (e) => { if (e.target !== e.currentTarget || (e.key !== "Enter" && e.key !== " ")) return; e.preventDefault(); if (selected && e.key === "Enter") onOpen(r.id); else onPick(r.id); } },
    markEl(r.mark, r.color, r.markDim),
    h("span.sw-main",
      // The tag has a column of its own, so what the task says and its bar start under the title, not under the tag.
      r.tag ? h("span.sw-id", { style: `min-width:${r.tagWidth || r.tag.length}ch` }, r.tag) : null,
      h("span.sw-line", h(`span.sw-title${r.strong ? ".sw-b" : ""}`, titleOf(r)), h("span.sw-time", r.time)),
      // An automation's word (on, off) stands first in what it says where the list is too narrow for the tags' column.
      says || r.automation ? h("span.sw-says", r.automation && r.tag ? h("span.sw-word", `${r.tag} · `) : null, ink(r.says)) : null,
      meter || tags.childNodes.length ? h("span.sw-under", meter, tags) : null,
      selected && !opened ? openEl(r, onOpen) : null));
}
// Opening is a step of its own: the selected row's button, never the tap that selected it (none while it is open).
const openEl = (r, onOpen) => h("span.sw-row-acts", h("button.btn.sw-open", { type: "button", "aria-label": `Open ${r.tag || r.title}`, onclick: (e) => { e.stopPropagation(); onOpen(r.id); } }, r.automation ? "Open automation" : "Open task", icon("right")));

// Words on their way to another machine: a row of their own until that machine has taken them.
const jobEl = (j, { selected, onPick }) => h("li.sw-row", { className: `sw-row${selected ? " is-selected" : ""}`, data: { id: j.id }, role: "option", "aria-selected": selected ? "true" : "false", tabIndex: 0, onclick: () => onPick(j.id), onkeydown: (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onPick(j.id); } } },
  markEl(j.failed ? "✗" : "●", j.failed ? "red" : "yellow"),
  h("span.sw-main", h("span.sw-line", h("span.sw-title", j.text)), j.failed && j.msg ? h("span.sw-says", j.msg) : null));

// Do the tags fit a list this wide (in pixels)? As on the terminal board (view/row-number.js: NAME_LEFT), they give
// way on every row at once where the widest would leave its title fewer than 16 letters: the row's padding, the mark,
// the tag, the title and the time at the widths the styles give them (board.css).
const NAME_LEFT = 16, PAD = 24, MARK_W = 30, TAG_CH = 7.3, TAG_GAP = 8, TITLE_CH = 7.5, TIME_CH = 6.6, TIME_GAP = 12;
export const tagsFit = (px, widest, time = 0) => !widest || px <= 0 || px >= PAD + MARK_W + widest * TAG_CH + TAG_GAP + NAME_LEFT * TITLE_CH + (time ? TIME_GAP + time * TIME_CH : 0);

export function listEl(list, opts) {
  const el = drawList(list, opts), rows = list.groups.flatMap((g) => g.rows), widest = Math.max(0, ...rows.map((r) => (r.tag || "").length));
  if (widest && typeof ResizeObserver !== "undefined") {
    const time = Math.max(0, ...rows.map((r) => (r.time || "").length));
    new ResizeObserver(() => el.classList.toggle("sw-no-tags", !tagsFit(el.clientWidth, widest, time))).observe(el);
  }
  return el;
}
// A heading: a machine's (over its groups) or a group's (over its rows), as the terminal board has them. A tap folds it
// to its one line, with how many tasks (and automations) it holds, and unfolds it.
// A coordinator's heading (and the main Coordinator's row on top) on autopilot is drawn as a task's row: autopilot's
// mark first, in the marks' column left of the caret, its columns on the right, what it does now on a line under it.
// The selected one has an Open button that shows the coordinator whole.
function headEl(g, selected, onFold, onOpen) {
  const count = g.main ? null : g.folded ? g.count : g.machine ? null : g.rows.length + g.more;
  const p = g.pilot;
  // A repo's heading has its key beside its name, dimmed: `gtm-agent · AGT`.
  const mark = p ? h("span.sw-pilot-mark", { "aria-label": "on autopilot" }, ink(p.mark)) : null;
  const words = [g.key ? h("span", g.name, h("span.sw-repo-key", ` · ${g.key}`)) : g.name,
    count === null ? null : h("span.sw-count", String(count)), g.sum ? h("span.sw-count", ` · ${g.sum}`) : null];
  const tag = g.machine || g.main ? "h2.sw-machine-name" : "h3.sw-group-name";
  const cols = p?.cols?.length ? h("span.sw-pilot-cols", p.cols.map((c) => ink(c))) : null;
  const under = p ? h("span.sw-pilot-says", ink(p.under)) : null;
  if (!g.id) return h(`${tag}.sw-c-${g.color}${p ? ".sw-on-pilot" : ""}`, mark, words, cols, under);
  const open = g.coord && selected ? h("button.btn.sw-open", { type: "button", "aria-label": `Open ${g.name}`, onclick: (e) => { e.stopPropagation(); onOpen(g.id); } }, "Open", icon("right")) : null;
  return h(`${tag}.sw-c-${g.color}${selected ? ".is-selected" : ""}${g.main ? ".sw-main-row" : ""}${p ? ".sw-on-pilot" : ""}`,
    h("button.sw-fold", { type: "button", "aria-expanded": g.folded ? "false" : "true", title: g.main ? "The Coordinator" : g.folded ? "Unfold" : "Fold", onclick: () => onFold(g.id) }, mark, g.main ? null : h("span.sw-caret", { "aria-hidden": "true" }, g.folded ? "▸" : "▾"), words), cols, open, under);
}
// Machines whose tasks could not be read: one notice over the list, each machine a line with what to do behind it.
// Dismissed, it stays away in this browser until another machine stops answering or one answers for another reason.
const GONE = "sw-unread-off";
const said1 = (u) => `${u.machine}\n${u.line}`;
const gone = () => { try { return JSON.parse(localStorage.getItem(GONE) || "[]"); } catch { return []; } };
function unreadEl(list) {
  const all = list.unread || [], bad = all.filter((u) => !u.asking), asking = all.filter((u) => u.asking);
  const off = gone(), show = bad.length && !bad.every((u) => off.includes(said1(u)));
  const names = bad.map((u) => u.machine), who = names.length === 1 ? names[0] : `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
  const box = show ? h("div.sw-unread", { role: "status" },
    h("div.sw-unread-top", h("p.sw-unread-head", `${who} ${names.length === 1 ? "does" : "do"} not answer, so ${names.length === 1 ? "its" : "their"} tasks are not shown. The board keeps asking by itself.`),
      h("button.icon-button.sw-unread-x", { type: "button", "aria-label": "Dismiss", title: "Dismiss", onclick: () => { try { localStorage.setItem(GONE, JSON.stringify(bad.map(said1))); } catch {} box.remove(); } }, icon("close"))),
    bad.map((u) => h("div.sw-unread-one", h("p.sw-unread-line", u.line),
      u.detail.length ? h("details", h("summary", "What to do"), h("pre.sw-unread-detail", u.detail.join("\n"))) : null))) : null;
  return [box, ...asking.map((u) => h("p.sw-asking", { role: "status" }, u.line))].filter(Boolean);
}
function drawList(list, { opened, onPick, onOpen, onMore, onFold }) {
  const unread = unreadEl(list);
  if (!list.groups.length && !list.jobs.length) return unread.length ? h("div.sw-list", unread, list.empty ? h("p.sw-empty", list.empty) : null) : h("p.sw-empty", list.empty);
  const jobs = list.jobs.length ? h("section.sw-group", h("h3.sw-group-name.sw-c-overlay0", "on their way"), h("ul.sw-rows", list.jobs.map((j) => jobEl(j, { selected: j.id === list.selected, onPick })))) : null;
  return h("div.sw-list", { role: "listbox", "aria-label": "Tasks" }, unread, jobs, list.groups.map((g) => g.machine || g.main ? headEl(g, g.id === list.selected, onFold, onOpen) : h("section.sw-group",
    headEl(g, g.id === list.selected, onFold, onOpen),
    h("ul.sw-rows", g.rows.map((r) => rowEl(r, { selected: r.id === list.selected, opened: r.id === opened, onPick, onOpen }))),
    g.more ? h("button.btn.btn-ghost.sw-more", { type: "button", onclick: () => onMore(g) }, `Show ${g.more} more`) : null)));
}
