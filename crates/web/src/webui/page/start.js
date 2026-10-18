// The script the front door adds to the herdr web UI's page. It puts a Boards section into the web UI's own sidebar
// (boards.js: under Agents, above its foot with Settings) and opens the board over the page. When that place is not
// found (the web UI was redesigned) a small floating Board button shows instead and says so once: the board is never
// just missing. Where the board's chat went full screen into the web UI's own chat view, a bar over that view goes back
// to the board and flips who you talk to (talk.js).
// Nothing shows until the web UI has let you in: the board's first call is refused before that, and is asked again.
/* global document, window, location, history, MutationObserver, localStorage, requestAnimationFrame */
import { h, icon, get } from "./dom.js";
import { openBoard, closeBoard, isOpen, freshen } from "./board.js";
import { boardsEl } from "./boards.js";
import { sync as syncFull, fullBar } from "./talk.js";

const HASH = "#board";
const FOOT = ".sidebar-footer";
let allowed = false, floating = null;

function styles() {
  if (document.querySelector("link[data-sw]")) return;
  for (const name of ["board", "chat", "compose", "files", "voice", "boards", "talk"]) document.head.append(h("link", { rel: "stylesheet", href: `/herdr-projects/page/${name}.css`, "data-sw": true }));
}
const entry = (cls) => { const b = h(`button.${cls}`, { type: "button", "data-sw-entry": true, title: "Work board", onclick: () => show(true) }); b.append(icon("board"), "Board"); return b; };
// A board picked in the sidebar: shown, and on a phone the web UI's drawer closes as it does when you pick a pane there.
function picked(out) {
  if (!out.ok) return;
  if (isOpen()) freshen(); else show(true);
  try { document.querySelector(".scrim.is-open")?.click(); } catch {}
}
// In the web UI's sidebar, right above its foot, when it is there; the floating button otherwise.
function place() {
  if (!allowed) return;
  const foot = document.querySelector(FOOT);
  if (foot) {
    const boards = boardsEl(picked);
    if (boards.nextElementSibling !== foot) foot.before(boards);
    floating?.remove(); floating = null;
    return;
  }
  if (floating || document.querySelector("[data-sw-entry]")) return;
  floating = entry("btn.sw-float");
  document.body.append(floating);
  try { if (!localStorage.getItem("sw-float-said")) { localStorage.setItem("sw-float-said", "1"); floating.append(h("span.sw-float-note", "The web UI's menu was not found, so the Board is here.")); setTimeout(() => floating?.querySelector(".sw-float-note")?.remove(), 12000); } } catch {}
}
// Open or close, and say so in the address: Back closes the board, and a link that ends in #board opens it.
// `full`: with its chat full screen.
function show(on, full = false) {
  if (on && location.hash !== HASH) history.pushState(history.state, "", location.pathname + location.search + HASH);
  if (!on && location.hash === HASH) history.replaceState(history.state, "", location.pathname + location.search);
  if (on) openBoard({ onClose: () => show(false), full }); else closeBoard();
  syncFull(isOpen());
}
function follow() { if (location.hash === HASH) { if (!isOpen()) openBoard({ onClose: () => show(false) }); } else if (isOpen()) closeBoard(); syncFull(isOpen()); }
// The web UI lays its page out again as it likes: the bar over its chat view follows, once a frame at most.
let laying = false;
const relay = () => { if (laying) return; laying = true; requestAnimationFrame(() => { laying = false; syncFull(isOpen()); }); };

async function begin() {
  const first = await get("hello");
  if (!first.ok) return void setTimeout(begin, first.status === 401 ? 3000 : 10000); // not logged in yet, or the door is restarting
  allowed = true;
  styles();
  place();
  new MutationObserver(() => { place(); relay(); }).observe(document.body, { childList: true, subtree: true });
  fullBar((full) => show(true, full));
  window.addEventListener("hashchange", follow);
  window.addEventListener("popstate", follow);
  follow();
}
begin();
