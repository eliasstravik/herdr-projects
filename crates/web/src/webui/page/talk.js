// Going into a pane: Open agent goes to the web UI's own view of a task's pane (or a coordinator's) from here.
/* global window, document, location, history, PopStateEvent, setTimeout */

const paneNow = () => new URLSearchParams(location.search).get("pane") || "";
function write(go, push) {
  const q = new URLSearchParams(location.search);
  for (const k of ["machine", "file", "settings", "section", "new", "cwd", "name", "agent", "sidebar"]) q.delete(k);
  if (go.machine) q.set("machine", go.machine); // on another of the web UI's PCs: its own name for it
  q.set("ws", go.workspace); q.set("pane", go.pane); q.set("view", go.view);
  history[push ? "pushState" : "replaceState"](null, "", `${location.pathname}?${q}`);
  window.dispatchEvent(new PopStateEvent("popstate", { state: null }));
}
// Does the web UI show that space here? Its sidebar names each space's row by a pane of it (`data-pane`, Ovis 0.4.13);
// a page with no such names, or a pane on another of its PCs (the same ids there), cannot say.
function shown(go) {
  if (go.machine) return true;
  const named = [...document.querySelectorAll("[data-pane]")];
  return !named.length || named.some((el) => el.dataset.pane.split(":")[0] === go.workspace);
}
// Into a pane: the web UI's own view of it, on this machine or on another of its PCs. Its address says where the app
// is, and it follows a change of address as it follows Back, so the page is not loaded again. The web UI drops an
// address naming a pane it does not know yet (Ovis 0.4.13 stays where it was, its address put back or not): the address
// is written again a few times, a second apart; when the web UI still did not go, its address is put back and
// `missed` is called. Somewhere else picked meanwhile (another pane, the board again) ends the tries.
export function goTo(go, missed) {
  const from = paneNow(), back = location.search;
  let tries = 0;
  const check = () => {
    const now = paneNow();
    if ((now === go.pane && shown(go)) || (now !== go.pane && now !== from) || location.hash) return;
    if (++tries > 3) { history.replaceState(null, "", `${location.pathname}${back}`); return missed?.(); }
    write(go, false);
    setTimeout(check, 1000);
  };
  write(go, true);
  if (from !== go.pane) setTimeout(check, 1000);
}
