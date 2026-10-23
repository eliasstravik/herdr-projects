// The task list as the page draws it: the groups under their headings and one row per task, each as far in as the
// terminal board sets it. Everything shown comes from the board (webui/model/list.rs): this file only lays it out. A tap on a row selects it, as the cursor does on
// the terminal board; the selected row has an Open button, which shows it whole.
/* global ResizeObserver, localStorage */
import { h, icon, ink, said, markEl } from "./dom.js";

// What is said of a task after its title (`×3`, `[auto]`, `[research]` …, the pin mark `⚑`) is dimmed: colour means status only.
const titleOf = (r) => { const m = /(?: ×\d+)?(?: \[[a-z]+\])*(?: ⚑)?$/.exec(r.title); const at = m && m[0] ? m.index : -1; return at < 0 ? r.title : [r.title.slice(0, at), h("span.sw-count", r.title.slice(at))]; };

export function rowEl(r, { selected, opened, onPick, onOpen }) {
  const tags = h("span.sw-tags", r.review?.length ? ink(r.review, "sw-tag") : null,
    r.pr?.length ? (r.prUrl ? h("a.sw-tag", { href: r.prUrl, target: "_blank", rel: "noopener noreferrer", onclick: (e) => e.stopPropagation() }, ink(r.pr)) : ink(r.pr, "sw-tag")) : null);
  // A task that waits for a free slot has its place in the queue (Q#2) where a task at work has its percent.
  const meter = r.progress === null && !r.eta && !r.percent ? null : h("span.sw-meter", { className: `sw-meter sw-c-${r.color}` },
    r.progress === null ? null : h("span.sw-bar", { role: "progressbar", "aria-valuenow": r.progress, "aria-valuemin": 0, "aria-valuemax": 100 }, h("span.sw-bar-fill", { style: `width:${r.progress}%` })),
    r.percent ? h("span.sw-num", r.percent) : null, r.eta ? h("span.sw-num", r.eta) : null);
  const says = said(r.says);
  // Enter on the selected row opens it, as its Open button does; on any other row it selects, as a tap does.
  return h("li.sw-row", { className: `sw-row${selected ? " is-selected" : ""}${r.quiet ? " is-quiet" : ""}${r.archived ? " is-archived" : ""}${r.automation ? " is-automation" : ""}`, data: { id: r.id }, style: `--d:${r.depth || 0}`, role: "option", "aria-selected": selected ? "true" : "false", tabIndex: 0,
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
// A heading, drawn as the terminal board draws it (board/view/list.rs): the top row (All machines, or the machine
// shown), a machine's, a project's, a status group's or the automations'. A machine's and a project's row is its
// coordinator's, read as a task's row: its mark in the marks' column (○ idle, ◐ working, ? asks you, ✗ could not start),
// its time on the right, and while on autopilot a dimmed [autopilot] after its name (in its words) and what autopilot
// does under it, in a task's grey second line (`Autopilot · next check-in in 12m · 3 of 20 today`). A status group's caret stands in the marks' column. Its
// words come in their inks: the name in the base colour (a machine's and the top row's bold), a status group's in its
// colour, what goes with it dimmed. A tap folds it to its one line, and unfolds it; the selected coordinator's row has
// an Open button that shows the coordinator whole.
function headEl(g, selected, { onFold, onOpen, onPick, actions = [], onPress }) {
  const p = g.pilot;
  const kind = g.top ? "top" : g.machine ? "machine" : g.repo ? "project" : g.automations ? "autos" : "group";
  // (Drawn as a task's mark is: the round ones at one size, dom.js.)
  const m = p?.mark?.find((x) => x.t.trim()) || null;
  const mark = p ? h("span.sw-head-mark", { role: "img", "aria-label": p.label || "no state" }, m ? markEl(m.t, m.c, !!m.d) : h("span.sw-mark")) : null;
  const words = h("span.sw-head-words", ink(g.words));
  const time = p?.cols?.length ? h("span.sw-time.sw-head-time", p.cols.map((c) => ink(c))) : null;
  const under = p?.under?.length ? h("span.sw-says.sw-head-says", ink(p.under)) : null;
  // Selected: Open shows its coordinator whole. A project with none to open (archived, in the archive) has its buttons
  // here instead (Restore this project).
  const open = !selected ? null : g.coord ? h("span.sw-head-acts", h("button.btn.sw-open", { type: "button", "aria-label": `Open ${g.name}`, onclick: (e) => { e.stopPropagation(); onOpen(g.id); } }, "Open", icon("right")))
    : actions.length ? h("span.sw-row-acts", actions.map((b) => h("button.btn.sw-open", { type: "button", onclick: (e) => { e.stopPropagation(); onPress?.(b.id); } }, b.label[0].toUpperCase() + b.label.slice(1)))) : null;
  const caret = (cls) => h(`button.sw-caret${cls}`, { type: "button", "aria-expanded": g.folded ? "false" : "true", "aria-label": `${g.folded ? "Unfold" : "Fold"} ${g.name}`, title: g.folded ? "Unfold" : "Fold", onclick: (e) => { e.stopPropagation(); onFold(g.id); } }, g.folded ? "▸" : "▾");
  if (!g.id) return h(`div.sw-hrow.is-${kind}`, { style: `--d:${g.depth || 0}` }, h("span.sw-hline", mark || h("span.sw-mark"), words, time));
  // A coordinator's row is selected by a tap, as a task's row is (its coordinator is then the other one to talk to),
  // and folds by its caret; a status group's folds by a tap anywhere on it, as it has nobody to talk to.
  if (!g.coord && !g.repo) return h(`div.sw-hrow.is-${kind}.is-fold${selected ? ".is-selected" : ""}`, { data: { id: g.id }, style: `--d:${g.depth || 0}`, onclick: () => onFold(g.id) }, h("span.sw-hline", caret(".sw-head-caret"), words, time));
  return h(`div.sw-hrow.is-${kind}${selected ? ".is-selected" : ""}`, { data: { id: g.id }, style: `--d:${g.depth || 0}`, role: "option", "aria-selected": selected ? "true" : "false", tabIndex: 0,
    onclick: () => { if (!selected) onPick(g.id); }, onkeydown: (e) => { if (e.target !== e.currentTarget || (e.key !== "Enter" && e.key !== " ")) return; e.preventDefault(); if (selected && e.key === "Enter") onOpen(g.id); else onPick(g.id); } },
    h("span.sw-hline", mark, caret(""), words, time, open?.classList.contains("sw-head-acts") ? open : null), under, open?.classList.contains("sw-head-acts") ? null : open);
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
function drawList(list, { opened, onPick, onOpen, onMore, onFold, actions, onPress }) {
  const unread = unreadEl(list);
  if (!list.groups.length && !list.jobs.length) return unread.length ? h("div.sw-list", unread, list.empty ? h("p.sw-empty", list.empty) : null) : h("p.sw-empty", list.empty);
  const jobs = list.jobs.length ? h("section.sw-group", h("div.sw-hrow.is-group", h("span.sw-hline", h("span.sw-mark"), h("span.sw-head-words.sw-c-overlay0", "on their way"))), h("ul.sw-rows", list.jobs.map((j) => jobEl(j, { selected: j.id === list.selected, onPick })))) : null;
  // Each heading over its rows, each row as far in as it is deep: all machines → machine → project → task.
  return h("div.sw-list", { role: "listbox", "aria-label": "Tasks" }, unread, jobs, list.groups.map((g) => h(`section.sw-group.is-${g.top ? "top" : g.machine ? "machine" : g.repo ? "project" : g.automations ? "autos" : "group"}`,
    headEl(g, g.id === list.selected, { onFold, onOpen, onPick, actions, onPress }),
    g.rows.length ? h("ul.sw-rows", g.rows.map((r) => rowEl(r, { selected: r.id === list.selected, opened: r.id === opened, onPick, onOpen }))) : null,
    g.more ? h("button.btn.btn-ghost.sw-more", { type: "button", style: `--d:${g.rows[0]?.depth ?? g.depth ?? 0}`, onclick: () => onMore(g) }, `Show ${g.more} done`) : null)));
}
