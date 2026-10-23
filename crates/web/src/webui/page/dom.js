// The page's few tools: making elements, drawing the board's coloured pieces of text, reading the web UI's own settings,
// and calling the front door.
/* global document, fetch, localStorage, sessionStorage, crypto, location */

// h("div.sw-row", { onclick }, child, …): a tag with classes, then properties and children.
export function h(spec, ...rest) {
  const [tag, ...classes] = spec.split(".");
  const el = document.createElement(tag || "div");
  if (classes.length) el.className = classes.join(" ");
  for (const x of rest.flat()) {
    if (x == null || x === false) continue;
    if (typeof x === "string" || typeof x === "number") el.append(String(x));
    else if (x.nodeType) el.append(x);
    else for (const [k, v] of Object.entries(x)) { if (v == null || v === false) continue; if (k.startsWith("on")) el.addEventListener(k.slice(2), v); else if (k === "data") Object.assign(el.dataset, v); else if (k in el && k !== "list") el[k] = v; else el.setAttribute(k, v === true ? "" : v); }
  }
  return el;
}
// The few icons, drawn as the web UI draws its own (24 by 24, a line of the text's colour).
const PATHS = {
  back: '<path d="m12 19-7-7 7-7"/><path d="M19 12H5"/>',
  find: '<path d="m21 21-4.34-4.34"/><circle cx="11" cy="11" r="8"/>',
  more: '<circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/>',
  close: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
  send: '<path d="m5 12 7-7 7 7"/><path d="M12 19V5"/>',
  plus: '<path d="M5 12h14"/><path d="M12 5v14"/>',
  mic: '<path d="M12 19v3"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><rect x="9" y="2" width="6" height="13" rx="3"/>',
  stop: '<rect width="14" height="14" x="5" y="5" rx="2"/>',
  check: '<path d="M20 6 9 17l-5-5"/>',
  file: '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/>',
  up: '<path d="m18 15-6-6-6 6"/>',
  down: '<path d="m6 9 6 6 6-6"/>',
  eye: '<path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"/><circle cx="12" cy="12" r="3"/>',
  eyeOff: '<path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49"/><path d="M14.084 14.158a3 3 0 0 1-4.242-4.242"/><path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143"/><path d="m2 2 20 20"/>',
  right: '<path d="m9 18 6-6-6-6"/>',
  swap: '<path d="M8 3 4 7l4 4"/><path d="M4 7h16"/><path d="m16 21 4-4-4-4"/><path d="M20 17H4"/>',
  expand: '<path d="M15 3h6v6"/><path d="m21 3-7 7"/><path d="m3 21 7-7"/><path d="M9 21H3v-6"/>',
  shrink: '<path d="m14 10 7-7"/><path d="M20 10h-6V4"/><path d="m3 21 7-7"/><path d="M4 14h6v6"/>',
  all: '<path d="M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"/><path d="m22 17.65-9.17 4.16a2 2 0 0 1-1.66 0L2 17.65"/><path d="m22 12.65-9.17 4.16a2 2 0 0 1-1.66 0L2 12.65"/>',
  board: '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 3v18"/><path d="M13 8h4"/><path d="M13 12h4"/><path d="M13 16h2"/>',
  phone: '<path d="M13.832 16.568a1 1 0 0 0 1.213-.303l.355-.465A2 2 0 0 1 17 15h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2A18 18 0 0 1 2 4a2 2 0 0 1 2-2h3a2 2 0 0 1 2 2v3a2 2 0 0 1-.8 1.6l-.468.351a1 1 0 0 0-.292 1.233 14 14 0 0 0 6.392 6.384"/>',
  phoneOff: '<path d="M10.1 13.9a14 14 0 0 0 3.732 2.668 1 1 0 0 0 1.213-.303l.355-.465A2 2 0 0 1 17 15h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2 18 18 0 0 1-12.728-5.272"/><path d="M22 2 2 22"/><path d="M4.76 13.582A18 18 0 0 1 2 4a2 2 0 0 1 2-2h3a2 2 0 0 1 2 2v3a2 2 0 0 1-.8 1.6l-.468.351a1 1 0 0 0-.292 1.233 14 14 0 0 0 .244.473"/>',
  micOff: '<path d="M12 19v3"/><path d="M15 9.34V5a3 3 0 0 0-5.68-1.33"/><path d="M16.95 16.95A7 7 0 0 1 5 12v-2"/><path d="M18.89 13.23A7 7 0 0 0 19 12v-2"/><path d="m2 2 20 20"/><path d="M9 9v3a3 3 0 0 0 5.12 2.12"/>',
};
export function icon(name) {
  const span = document.createElement("span");
  span.className = "sw-icon";
  span.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${PATHS[name] || ""}</svg>`;
  return span.firstChild;
}
// The board's text as it was painted there: each piece with the name of its colour, which the styles give a colour.
export function ink(pieces, cls = "") {
  const out = h(`span${cls ? "." + cls : ""}`);
  for (const p of pieces || []) {
    const cls = [p.c ? `sw-c-${p.c}` : "", p.b ? "sw-b" : "", p.d ? "sw-d" : ""].filter(Boolean).join(" ");
    out.append(p.href ? h("a", { className: cls, href: p.href, target: "_blank", rel: "noopener noreferrer" }, p.t) : cls ? h("span", { className: cls }, p.t) : p.t);
  }
  return out;
}
// A status mark. The round ones (an agent at work, its reviewer, a task that waits for a free slot) are drawn, all at
// one size: as text the half disc came out a third the size of the dot, and the dotted ring is missing from many
// fonts. Every other mark is its character (the board's own step ◇ beside the ◆ of ready).
// So is an automation's arrow going round (⟳), at the size of the round ones in any font; off, it is the same one, quiet.
const ROUND = { "●": "full", "◑": "right", "◌": "dotted" }, TURN = "⟳";
export const markEl = (mark, color = "", dim = false) => h(`span.sw-mark${color ? `.sw-c-${color}` : ""}${dim ? ".sw-d" : ""}`, { "aria-hidden": "true" }, ROUND[mark] ? h(`span.sw-dot.sw-dot-${ROUND[mark]}`) : mark === TURN ? h("span.sw-turn") : mark);
export const said = (pieces) => (pieces || []).map((p) => p.t).join("");
// The web UI's own settings, where its page keeps them: `ovis:settings` since its rename to Ovis (which moves the old
// record over), `herdr-web-ui:settings` before. None kept, or none that can be read: {}.
const SETTINGS = ["ovis:settings", "herdr-web-ui:settings"];
export function webUiSettings() {
  for (const key of SETTINGS) { try { const s = JSON.parse(localStorage.getItem(key) || "null"); if (s && typeof s === "object") return s; } catch {} }
  return {};
}

// This browser tab's own board (its selection, its question): a name of its own, kept while the tab lives.
export const client = (() => {
  try { let id = sessionStorage.getItem("sw-client"); if (!id) { id = crypto.randomUUID().replace(/-/g, "").slice(0, 16); sessionStorage.setItem("sw-client", id); } return id; }
  catch { return Math.random().toString(36).slice(2, 14); }
})();
const BASE = "/herdr-projects/api";
// The PC the web UI's page is on (its address says: `?machine=<pc>`, left out for this machine). Sent with every
// call, so the board is the board of that machine. While the board's chat is docked on an agent's chat view (dock.js),
// the page is on that agent's PC only for the chat: the board stays the board of the PC the page was on (`setHome`).
let home = null;
export const setHome = (pc) => { home = pc; };
const machine = () => { if (home !== null) return home; try { return new URLSearchParams(location.search).get("machine") || ""; } catch { return ""; } };
// A read. Answers { ok, status, … }; never throws.
export async function get(what, params = {}) {
  try {
    const r = await fetch(`${BASE}/${what}?${new URLSearchParams({ client, machine: machine(), ...params })}`, { credentials: "same-origin", cache: "no-store" });
    const j = await r.json().catch(() => ({}));
    return { ...j, ok: r.ok && j.ok !== false, status: r.status };
  } catch { return { ok: false, status: 0, msg: "No connection." }; }
}
// A change: the same address, with the header that says it comes from the board's own controls.
export async function post(what, body = {}) {
  try {
    const r = await fetch(`${BASE}/${what}?${new URLSearchParams({ client, machine: machine() })}`, { method: "POST", credentials: "same-origin", headers: { "content-type": "application/json", "x-herdr-projects": "1" }, body: JSON.stringify(body) });
    const j = await r.json().catch(() => ({}));
    return { ...j, ok: r.ok && j.ok !== false, status: r.status };
  } catch { return { ok: false, status: 0, msg: "No connection." }; }
}
