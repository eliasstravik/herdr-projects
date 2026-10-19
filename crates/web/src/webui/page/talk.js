// Who you talk to, and going into a pane. The names on the switch that flips who gets your words (the Coordinator, or
// the selected task's agent, as space does on the terminal board) come from here, for the bar of the board's chat
// (chat.js); and Open agent goes to the web UI's own view of a task's pane from here.
/* global window, location, history, PopStateEvent */

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
// Into a pane: the web UI's own view of it, on this machine or on another of its PCs. Its address says where the app
// is, and it follows a change of address as it follows Back, so the page is not loaded again.
export function goTo(go) {
  const q = new URLSearchParams(location.search);
  for (const k of ["machine", "file", "settings", "section", "new", "cwd", "name", "agent", "sidebar"]) q.delete(k);
  if (go.machine) q.set("machine", go.machine); // on another of the web UI's PCs: its own name for it
  q.set("ws", go.workspace); q.set("pane", go.pane); q.set("view", go.view);
  history.pushState(null, "", `${location.pathname}?${q}`);
  window.dispatchEvent(new PopStateEvent("popstate", { state: null }));
}
