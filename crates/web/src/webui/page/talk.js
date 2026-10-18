// Who you talk to, and the chat full screen in the web UI's own chat view. The names on the one button that flips who
// gets your words (the Coordinator, or the selected task's agent, as space does on the terminal board) come from here,
// for the board's chat (chat.js) and for the bar the page puts over the web UI's chat view when the board's chat went
// full screen into it: a back arrow to the board and that same button, which flips to the other one's chat view.
/* global document, window, location, history, sessionStorage, PopStateEvent, setTimeout */
import { h, icon, get, post } from "./dom.js";

// Who the chat names now (`is`) and who a tap flips to (`other`, "" for nobody), from the board's own answer.
export function talkNames(snap) {
  const c = snap?.chat;
  if (!c) return { is: "", other: "" };
  // Only on a task's row: on an automation's row that key is the automation's own (it turns it on or off). On a
  // machine's or a repo's heading the other choice is its own coordinator (`head`).
  const onTask = !!snap.list?.selected && snap.flags?.onTask !== false && !!c.task, onHead = !onTask && !!c.head;
  const coord = c.coord || c.name, agent = onTask ? `${c.task} agent` : onHead ? c.head : c.name;
  const [is, other] = c.toAgent ? [agent, coord] : [coord, onTask || onHead ? agent : ""];
  return { is, other };
}
// The button's insides and words: the arrows only when a tap flips to somebody.
export function drawTalk(btn, { is, other }) {
  btn.replaceChildren(...[other ? icon("swap") : null, h("span.sw-talk-name", is)].filter(Boolean));
  btn.disabled = !other; btn.title = other ? `Talk to ${other} instead` : "Select a task to talk to its agent";
  btn.setAttribute("aria-label", `Talking to ${is}. ${btn.title}.`);
}

// Into a pane: the web UI's own view of it, on this machine or on another of its PCs. Its address says where the app
// is, and it follows a change of address as it follows Back, so the page is not loaded again. `replace`: in place of
// the view that shows now (a flip in full screen), so Back still goes to the board.
export function goTo(go, replace = false) {
  const q = new URLSearchParams(location.search);
  for (const k of ["machine", "file", "settings", "section", "new", "cwd", "name", "agent", "sidebar"]) q.delete(k);
  if (go.machine) q.set("machine", go.machine); // on another of the web UI's PCs: its own name for it
  q.set("ws", go.workspace); q.set("pane", go.pane); q.set("view", go.view);
  history[replace ? "replaceState" : "pushState"](null, "", `${location.pathname}?${q}`);
  window.dispatchEvent(new PopStateEvent("popstate", { state: null }));
}

// The full screen kept for this tab: which pane it went to, the names on the button, and how deep the history was
// once it got there (Back to the board is one step back while nothing else was opened since).
const KEY = "sw-full";
const kept = () => { try { const x = JSON.parse(sessionStorage.getItem(KEY) || "null"); return x && typeof x.pane === "string" ? x : null; } catch { return null; } };
const keep = (x) => { try { if (x) sessionStorage.setItem(KEY, JSON.stringify(x)); else sessionStorage.removeItem(KEY); } catch {} };
// Going full screen: the board's chat goes to this pane's chat view (`replace`: from another one's, after a flip).
export function goFull(go, snap, replace = false) {
  keep({ pane: go.pane, machine: go.machine || "", ...talkNames(snap), depth: replace ? kept()?.depth ?? history.length : history.length + 1 });
  goTo(go, replace);
}
const here = (x) => { const q = new URLSearchParams(location.search); return !!x && q.get("pane") === x.pane && (q.get("machine") || "") === x.machine; };

// The bar over the web UI's chat view. `onBoard(full)`: show the board (with its own chat full screen: true).
let bar = null, talk = null, busy = false, drawn = "", onBoard = () => {};
function place() {
  // At the top of the pane the web UI shows, under its header and its tabs; on top of its chat, under its drawer.
  const frame = document.querySelector(".pane-frame.is-current") || document.querySelector(".terminal-host"), r = frame?.getBoundingClientRect();
  const head = document.querySelector(".app-header")?.getBoundingClientRect();
  bar.style.top = `${Math.round(r && r.height ? r.top : head ? head.bottom : 0)}px`;
  bar.style.left = `${Math.round(r && r.width ? r.left : 0)}px`;
}
function back() {
  const x = kept(); keep(null); sync();
  if (x && x.depth === history.length && here(x)) history.back(); else onBoard(false);
}
// A tap flips to the other one, as on the board, and shows their chat view in place of this one. Where the web UI
// cannot show them (no agent on the task yet, another machine's Coordinator), the board shows them full screen.
async function flip() {
  if (busy || talk.disabled) return;
  busy = true; bar.classList.add("is-busy");
  const was = (await get("board")).go?.n || 0;
  let out = await post("press", { id: "talk" });
  if (out.ok) out = await post("full");
  // Whether the board will link to their chat view is said once, by `full`; the link itself may come later: the
  // agent's space is opened again first (on another machine always after the answer), and a later look brings it.
  const asked = out.ok && !!out.full;
  let link = asked && out.go?.n > was ? out : null;
  for (let i = 0; asked && !link && i < 50; i++) { await new Promise((r) => setTimeout(r, 600)); const look = await get("board"); if (look.ok && look.go?.n > was) link = look; }
  busy = false; bar.classList.remove("is-busy");
  if (link) return goFull(link.go, link, true);
  keep(null); sync(); onBoard(true);
}
// Shown while the web UI shows the pane the board's chat went to, and the board is closed.
export function sync(open = false) {
  const x = kept(), on = !open && here(x);
  document.documentElement.classList.toggle("sw-full-on", on);
  if (!on) { if (bar) bar.hidden = true; return; }
  if (!bar) {
    talk = h("button.sw-full-talk", { type: "button", onclick: flip });
    bar = h("nav.sw-full-bar", { "aria-label": "Board chat" },
      h("button.sw-full-back", { type: "button", "aria-label": "Back to the board", title: "Back to the board", onclick: back }, icon("back")), talk);
    // Tapping it never takes the focus from the web UI's own chat box: a phone keeps its keyboard.
    bar.addEventListener("pointerdown", (e) => e.preventDefault());
    document.body.append(bar);
    window.addEventListener("resize", () => { if (bar && !bar.hidden) place(); });
  }
  // (Drawn again only when the names change: the page watches its own changes to keep the bar in place.)
  const names = JSON.stringify([x.is, x.other]);
  if (names !== drawn) { drawn = names; drawTalk(talk, x); }
  bar.hidden = false;
  place();
}
export function fullBar(show) { onBoard = show; }
