// Going into a pane: Open agent goes to the web UI's own view of a task's pane (or a coordinator's) from here.
/* global window, location, history, PopStateEvent */

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
