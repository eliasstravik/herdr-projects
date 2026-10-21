// The board's chat docked on the web UI's own chat: while the board is open, the web UI under it shows the chat view of
// whoever the board's chat talks to (the Coordinator, a heading's coordinator, the selected task's agent), and the
// board covers only the top of the page: its header, its list and the bar over the chat (chat.js). What you read and
// type in is the web UI's own transcript and box, with its markdown, tools, files, dictation and Stop, and the board's
// Call button put in that box beside its mic (callbtn.js), put back there whenever the web UI draws its box again. The board says
// where that chat is (its `dock` call); the page goes there in place (the address keeps #board, and Back still closes
// the board) and moves the web UI's pane column out of the board's way: down under the list, or to the right of it on a
// wide screen (board.css). Where the web UI has nobody to show, or its page is not the one this was made for, the
// board's own chat stands in. Closing the board puts the web UI back on the pane it showed before.
/* global document, window, location, history, PopStateEvent, MutationObserver, ResizeObserver, requestAnimationFrame,
   setTimeout, clearTimeout, HTMLTextAreaElement, Event */
import { setHome } from "./dom.js";
import { sendsOn } from "./compose.js";
import { keepList, STEPS } from "./keep.js";

// The web UI's places this leans on (herdr-web-ui 0.4.12: App.tsx, PaneTerminal.tsx, Composer.tsx there).
const COLUMN = ".pane-column", CHAT = ".pane-frame.is-current .terminal-stack.is-chat", COMPOSER = ".pane-frame.is-current .composer";
const BOX = "textarea.composer-text";
// Its box's left controls: the add button, then the mic where dictation is on (VoiceInput.tsx there).
const LEFT = ".composer-controls-left", AFTER = [":scope > .voice-mic-wrap", ":scope > .composer-attach"];
// How long the web UI may take to show the chat view it was sent to before the board's own chat stands in.
const WAIT_MS = 4000;
const html = document.documentElement;

const here = (go) => { const q = new URLSearchParams(location.search); return q.get("pane") === go.pane && (q.get("machine") || "") === (go.machine || ""); };
// To that pane's chat view, in place of what the page shows: the web UI follows a change of address as it follows Back.
function goHere(go) {
  const q = new URLSearchParams(location.search);
  for (const k of ["machine", "file", "settings", "section", "new", "cwd", "name", "agent", "sidebar"]) q.delete(k);
  if (go.machine) q.set("machine", go.machine);
  q.set("ws", go.workspace); q.set("pane", go.pane); q.set("view", "chat");
  history.replaceState(history.state, "", `${location.pathname}?${q}${location.hash}`);
  window.dispatchEvent(new PopStateEvent("popstate", { state: history.state }));
}
// The web UI keeps the view of a pane it already shows: its own Chat button then.
function chatView() {
  if (document.querySelector(CHAT)) return;
  const btn = document.querySelector(".view-switch button");
  if (btn && btn.getAttribute("aria-pressed") !== "true") btn.click();
}
// A value put into the web UI's box as typing would (its framework reads the input event, not the value).
function setBox(box, text) {
  Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(box, text);
  box.dispatchEvent(new Event("input", { bubbles: true }));
}
const theBox = () => document.querySelector(`${COMPOSER} ${BOX}`);
// The web UI's box has its completion menu open: the box says so (aria-expanded), and the menu stands in its card.
const menuOpen = (box) => box.getAttribute("aria-expanded") === "true" || !!box.closest(COMPOSER)?.querySelector(".composer-menu");
// A file in the web UI's box is still on its way (its tile says Uploading): its path is not in the words yet.
const uploading = (box) => !!box.closest(COMPOSER)?.querySelector(".composer-attachment.is-uploading");

// `boardOf()`: the board's element. `onChange()`: the web UI's chat came or went under the board, or its box changed
// height. `onTyping(on)`: a phone's keyboard is up for the web UI's box, or no longer. `route()`: how the words in the
// web UI's box go where the board sends them itself ({ send(text) → kept }), or null where the web UI sends them as it
// does (a task's agent). `guest`: the board's button put in the web UI's box, beside its mic, while docked. The Coordinator's go through the board, as its own box sends them: with where you were (the
// repo, the open task), and waiting while it is busy. It decides itself what to start, so there is no second way to send.
export function dockFor({ boardOf, onChange, onTyping, route, guest = null }) {
  let want = null, on = false, home = null, homeSearch = "", shown = "", wait = 0, typing = false, queued = false, boxH = -1, appH = "";
  const mine = () => !!want && here(want) && !!document.querySelector(CHAT) && !!document.querySelector(COLUMN);
  // What the board covers, given to the web UI's pane column as room to leave: down to the foot of the board's bar, or
  // on a wide screen the width of the board's column under the bar.
  const fit = () => {
    queued = false;
    const board = boardOf(), col = document.querySelector(COLUMN);
    if (!on || !board || !col) return;
    const b = board.getBoundingClientRect(), side = board.classList.contains("is-side");
    const bar = board.querySelector(".sw-chat")?.getBoundingClientRect();
    // (The column's own box: the room it leaves is inside it, so measuring again never moves it.)
    const c = col.getBoundingClientRect(), top = side ? (bar ? bar.bottom : b.top) : b.bottom;
    html.style.setProperty("--sw-dock-top", `${Math.max(0, Math.round(top - c.top))}px`);
    html.style.setProperty("--sw-dock-left", side ? `${Math.max(0, Math.round(b.right - c.left))}px` : "0px");
    const composer = document.querySelector(COMPOSER), ch = Math.round(composer?.getBoundingClientRect().height || 0);
    if (composer && !composer.dataset.swSized) { composer.dataset.swSized = "1"; sizes?.observe(composer); }
    // (And the height the web UI's shell leaves, which a phone's keyboard going down changes after the box let go.)
    const app = html.style.getPropertyValue("--app-height");
    if (ch !== boxH || app !== appH) { boxH = ch; appH = app; html.style.setProperty("--sw-composer-h", `${ch}px`); onChange(); }
    seat(composer);
    keys();
    // With the chat hidden the list stays in view, also while you type in the web UI's box: it gives way (keep.js).
    const box = theBox();
    if (box && board.classList.contains("sw-chat-hidden")) keepList(board, box, (n) => room(box, n), () => Math.max(0, board.scrollHeight - board.clientHeight), typing); else unkeep();
    marks.takeRecords(); // (what this measure changed on the page is its own: nothing to measure again for)
  };
  // (Made shorter with the caret at the end of the words, it scrolls to keep that line in view, as typing does.)
  const room = (box, n) => {
    if (!html.classList.contains("sw-web-room")) html.classList.add("sw-web-room");
    if (html.style.getPropertyValue("--sw-web-room") === `${n}px`) return;
    html.style.setProperty("--sw-web-room", `${n}px`);
    if (document.activeElement === box && box.selectionEnd >= box.value.length) box.scrollTop = box.scrollHeight;
  };
  const unkeep = () => {
    const board = boardOf();
    if (board && STEPS.some((c) => board.classList.contains(c))) board.classList.remove(...STEPS);
    if (html.classList.contains("sw-web-room")) { html.classList.remove("sw-web-room"); html.style.removeProperty("--sw-web-room"); }
  };
  // The guest right after the mic (after the add button where there is no mic); moved only when it is not there, so the
  // move it makes is seen once and changes nothing more. The web UI's own buttons are never moved.
  const seat = (composer) => {
    const left = guest && composer?.querySelector(LEFT);
    if (!left) return;
    const after = AFTER.map((q) => left.querySelector(q)).find(Boolean);
    const next = after ? after.nextSibling : left.firstChild;
    if (next !== guest) left.insertBefore(guest, next);
  };
  const soon = () => { if (!queued) { queued = true; requestAnimationFrame(fit); } };
  // Typing in the web UI's box with a phone's keyboard up (the web UI marks the page): the board steps aside to its bar.
  const keys = () => {
    const box = document.activeElement, now = on && !!box?.matches?.(BOX) && !!box.closest(COMPOSER) && html.hasAttribute("data-keyboard");
    if (now !== typing) { typing = now; onTyping(now); soon(); }
  };
  const set = (now) => {
    if (now === on) return;
    on = now; boxH = -1; appH = "";
    html.classList.toggle("sw-dock", on);
    if (!on) { for (const v of ["--sw-dock-top", "--sw-dock-left", "--sw-composer-h"]) html.style.removeProperty(v); unkeep(); if (typing) { typing = false; onTyping(false); } }
    onChange();
    soon();
  };
  // The web UI drew its page again: still the chat it was sent to? (Another pane picked in its own sidebar is not.)
  const look = () => {
    if (!want) return;
    if (mine()) { clearTimeout(wait); wait = 0; set(true); soon(); return; }
    if (on && !here(want)) { want = null; shown = ""; set(false); }
  };
  const marks = new MutationObserver(() => { look(); soon(); });
  const sizes = typeof ResizeObserver === "function" ? new ResizeObserver(soon) : null;
  window.addEventListener("resize", soon);
  window.visualViewport?.addEventListener("resize", soon);
  const blur = () => setTimeout(soon);
  document.addEventListener("focusin", soon); document.addEventListener("focusout", blur); document.addEventListener("input", soon, true);
  // Enter (as the web UI's Settings → Chat says) and its Send, for the Coordinator: sent by the board, before the web
  // UI's own handlers, which never see them then. Tab is the web UI's own.
  const onKey = (e) => {
    if (!on) return;
    const box = e.target;
    if (!sendsOn(e) || !box?.matches?.(BOX) || !box.closest(COMPOSER) || !box.value.trim()) return;
    // The web UI's own menu (its `/` commands, its `@` files) is open: Enter picks from it, as it does there.
    if (menuOpen(box)) return;
    const r = route();
    if (!r) return;
    // Nothing goes while a file is on its way, as the web UI's own Send waits: the keys do nothing until it is there.
    e.preventDefault(); e.stopPropagation();
    if (!uploading(box)) sendVia(box, r);
  };
  const onSend = (e) => {
    if (!on || !e.target?.closest?.(`${COMPOSER} .composer-send`)) return;
    const box = theBox(), r = route();
    if (!box || !r || !box.value.trim()) return;
    e.preventDefault(); e.stopPropagation();
    if (!uploading(box)) sendVia(box, r);
  };
  document.addEventListener("keydown", onKey, true);
  document.addEventListener("click", onSend, true);
  // Sent by the board, as its own box sends: the words leave the web UI's box once the board took them, and with them
  // the files they carried (their tiles, by the web UI's own remove, as its own Send lets them go). Files attached
  // while they were on their way stay, for the next words.
  let sending = false;
  async function sendVia(box, r) {
    if (sending || uploading(box)) return;
    const text = box.value, tiles = [...(box.closest(COMPOSER)?.querySelectorAll(".composer-attachment.is-ready") || [])];
    sending = true;
    const kept = await r.send(text);
    sending = false;
    if (!kept) {
      if (box.value === text) setBox(box, ""); // kept: the board did not take them, so they stay
      for (const t of tiles) if (t.isConnected) t.querySelector("button")?.click();
    }
    soon();
  }
  return {
    // Dock this chat view (null: none, the board's own chat stands in).
    show(go) {
      const key = go ? JSON.stringify([go.machine || "", go.pane]) : "";
      if (key === shown) return;
      shown = key;
      clearTimeout(wait); wait = 0;
      if (!go) { want = null; set(false); return; }
      if (home === null) { home = new URLSearchParams(location.search).get("machine") || ""; homeSearch = location.search; setHome(home); }
      want = go;
      if (!here(go)) goHere(go);
      marks.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["class", "aria-pressed"] });
      marks.observe(html, { attributes: true, attributeFilter: ["data-keyboard", "style"] });
      requestAnimationFrame(() => { chatView(); look(); });
      // Not shown in time: the board's own chat.
      wait = setTimeout(() => { wait = 0; if (!mine()) { want = null; set(false); } }, WAIT_MS);
    },
    on: () => on,
    fit: soon,
    // The board closed: the web UI back on the pane it showed before, if it still shows the one the board went to.
    destroy() {
      marks.disconnect(); sizes?.disconnect(); clearTimeout(wait);
      window.removeEventListener("resize", soon); window.visualViewport?.removeEventListener("resize", soon);
      document.removeEventListener("focusin", soon); document.removeEventListener("focusout", blur); document.removeEventListener("input", soon, true);
      document.removeEventListener("keydown", onKey, true); document.removeEventListener("click", onSend, true);
      for (const c of document.querySelectorAll("[data-sw-sized]")) delete c.dataset.swSized;
      if (guest?.closest(".composer")) guest.remove(); // nothing of the board's stays in the web UI's box
      const back = !!want && here(want) && home !== null;
      want = null; set(false); setHome(null);
      if (back && location.search !== homeSearch) { history.replaceState(history.state, "", `${location.pathname}${homeSearch}${location.hash}`); window.dispatchEvent(new PopStateEvent("popstate", { state: history.state })); }
    },
  };
}
