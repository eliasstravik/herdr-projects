// The box you type in, built as the herdr web UI builds its own chat input (its composer's card: Composer.tsx and
// Composer.css there), from the same CSS variables and none of its code: a card with the message as its first row,
// growing with the text, the files attached above it (files.js), and one row of controls under it, the add button, the
// mic (voice.js) and who the words are for on the left and the round Send on the right. Its grip, its keys and its sizes are the web
// UI's (docs/web-chat-box.md says how that was measured). What it does with the words is chat.js's.
/* global window, document, localStorage, getComputedStyle, setTimeout, clearTimeout */
import { h, icon } from "./dom.js";
import { filesEl } from "./files.js";
import { voiceEl } from "./voice.js";

// The web UI's own numbers: a key's step on the grip, how far a press must travel to be a drag, two taps that reset.
// And how long a tap on Send waits for a keyboard to finish the word it is composing.
const STEP = 24, SLACK = { mouse: 3, touch: 10 }, TWICE = 350, MOST = 480, LONGEST = 20000, COMMIT_WAIT = 300;
const HEIGHT = "sw-box-height";
const savedHeight = () => { try { const n = Number(localStorage.getItem(HEIGHT)); return n > 0 ? n : null; } catch { return null; } };
const keepHeight = (n) => { try { if (n == null) localStorage.removeItem(HEIGHT); else localStorage.setItem(HEIGHT, String(n)); } catch {} };
// Enter sends and shift+enter is a new line, on a phone too, unless the web UI's own Settings → Chat says that
// enter is a new line (then ctrl or cmd with enter sends). Read from where the web UI keeps it; if it ever keeps it
// elsewhere, enter sends.
export const enterSends = () => { try { return JSON.parse(localStorage.getItem("herdr-web-ui:settings") || "{}").enterSends !== false; } catch { return true; } };
export const sendsOn = (e, enter = enterSends()) => e.key === "Enter" && !e.isComposing && e.keyCode !== 229 && !e.shiftKey && (enter ? !e.metaKey && !e.ctrlKey : !!(e.metaKey || e.ctrlKey));

// `onSend(delegate)`: send what the box holds (true: and put an agent on what it adds). `onInput()`: its words changed.
// `onTab()`: tab was pressed with words in the box; true when it sent them (then the focus stays in the box).
// `whom()`: who an attached file is for, `hold()`: its path must wait (files.js). `onGrow()`: its height was set again
// (the room above a phone's keyboard may hold less: room.js).
export function composeEl({ onSend, onInput, onTab = () => false, whom = () => null, hold = () => false, onGrow = () => {} }) {
  let manual = savedHeight(), tapped = 0;
  const box = h("textarea.sw-input", { rows: 1, maxLength: LONGEST, spellcheck: false, autocapitalize: "off", autocorrect: "off", "aria-label": "Message" });
  const grip = h("div.sw-grip", { tabIndex: 0, role: "separator", "aria-orientation": "horizontal", "aria-label": "Resize message box", title: "Drag to resize · double-click to reset" });
  const to = h("span.sw-to", { hidden: true });
  const also = h("button.sw-also", { type: "button", hidden: true }, "Send + start");
  const send = h("button.sw-action.sw-send", { type: "button", disabled: true, "aria-label": "Send message" }, icon("send"));
  // Why a file did not go, or that some of a pick were left out: a band over the card, as the web UI's.
  const note = h("p.sw-box-note", { hidden: true, role: "status" });
  const say = (text) => { note.textContent = text || ""; note.hidden = !text; };
  const files = filesEl({ box, whom, hold, say, changed: () => { onInput(); grow(); } });
  // Dictated words go in as typed words do, without taking the focus.
  const voice = voiceEl({ box, say, write: (value, caret) => { box.value = value; box.setSelectionRange(caret, caret); onInput(); grow(); } });
  const card = h("div.sw-card.sw-b-text", grip, files.strip, box, h("div.sw-controls.sw-controls-left", files.input, files.add, voice.el, to), h("div.sw-controls.sw-controls-right", also, send));
  // As tall as its words, up to ten lines (the styles' limit), then it scrolls; or the height set with the grip.
  const grow = () => {
    box.classList.toggle("is-sized", manual != null); grip.classList.toggle("is-sized", manual != null);
    // (Measured at "auto", the field is one line for a moment: where it was scrolled to is put back after.)
    const top = box.scrollTop;
    if (manual != null) box.style.height = `${manual}px`;
    else { box.style.height = "auto"; box.style.height = `${box.scrollHeight}px`; }
    box.scrollTop = top;
    onGrow();
  };
  const floor = () => Math.round(parseFloat(getComputedStyle(box).minHeight)) || 44;
  const most = () => Math.max(floor(), Math.min(MOST, Math.round((window.visualViewport?.height || window.innerHeight) / 2)));
  // A grip height within the limits; at or under the automatic floor it is the automatic height again.
  const within = (n) => (n <= floor() ? null : Math.round(Math.min(most(), n)));
  const setHeight = (n, keep = true) => { manual = n; if (keep) keepHeight(n); grow(); };
  // Dragging the grip up grows the box. The pointer stays captured, so a finger may leave the grip.
  grip.addEventListener("pointerdown", (e) => {
    if (e.button > 0) return;
    e.preventDefault(); grip.setPointerCapture?.(e.pointerId);
    const slack = e.pointerType === "mouse" ? SLACK.mouse : SLACK.touch, from = box.getBoundingClientRect().height, y = e.clientY;
    let moved = false;
    const move = (m) => { if (!moved && Math.abs(m.clientY - y) < slack) return; moved = true; setHeight(within(from + y - m.clientY), false); };
    const end = (u) => {
      for (const [name, fn] of [["pointermove", move], ["pointerup", end], ["pointercancel", end]]) grip.removeEventListener(name, fn);
      if (moved) { tapped = 0; keepHeight(manual); } else if (u.type === "pointerup") { if (u.timeStamp - tapped < TWICE) { tapped = 0; setHeight(null); } else tapped = u.timeStamp; }
    };
    grip.addEventListener("pointermove", move); grip.addEventListener("pointerup", end); grip.addEventListener("pointercancel", end);
  });
  grip.addEventListener("keydown", (e) => {
    const now = manual ?? box.getBoundingClientRect().height, step = e.shiftKey ? STEP * 2 : STEP;
    if (e.key === "ArrowUp") setHeight(within(now + step)); else if (e.key === "ArrowDown") setHeight(within(now - step)); else if (e.key === "Home") setHeight(null); else return;
    e.preventDefault();
  });
  box.addEventListener("input", () => { say(null); onInput(); grow(); });
  box.addEventListener("keydown", (e) => {
    if (sendsOn(e)) { e.preventDefault(); onSend(false); return; }
    // Tab sends as the terminal board's does; with nothing typed, or where it does not apply, it moves on as tab does.
    if (e.key === "Tab" && !e.shiftKey && !e.altKey && !e.ctrlKey && !e.metaKey && !e.isComposing && box.value.trim() && onTab()) e.preventDefault();
  });
  files.dropOn(card);
  // Sending keeps the focus in the box, as the web UI's Send does: a button that took it would close a phone's
  // keyboard (and move out from under the tap before the click lands). So a press on either never takes the focus.
  // A tap while the keyboard still composes the last word (Gboard composes every word) finishes that word first: a
  // blur and a focus within the same tap commit it and keep the keyboard up, and the words go once it is committed.
  // A keyboard that has not committed it within the tap's time gets the tap withdrawn, and the box says so.
  let composing = false, waiting = null;
  const sendTapped = (delegate) => {
    clearTimeout(waiting?.timer); waiting = null;
    if (!composing) return onSend(delegate);
    const w = { delegate, timer: setTimeout(() => { if (waiting === w) { waiting = null; say("Finish the word, then send."); } }, COMMIT_WAIT) };
    waiting = w;
    if (document.activeElement === box) { box.blur(); box.focus({ preventScroll: true }); }
  };
  box.addEventListener("compositionstart", () => { composing = true; });
  box.addEventListener("compositionend", () => {
    composing = false;
    // WebKit puts the committed word in with an input event after compositionend: it is sent after that.
    const w = waiting;
    if (w) { clearTimeout(w.timer); waiting = null; setTimeout(() => { if (!composing) onSend(w.delegate); }); }
  });
  for (const [b, delegate] of [[send, false], [also, true]]) {
    b.addEventListener("pointerdown", (e) => e.preventDefault());
    b.addEventListener("click", () => sendTapped(delegate));
  }
  const fit = () => { if (manual != null && manual > most()) grow(); };
  window.addEventListener?.("resize", fit);
  const destroy = () => { window.removeEventListener?.("resize", fit); voice.destroy(); };
  return { el: h("div.sw-composer", { role: "group", "aria-label": "Message composer" }, note, voice.pill, card), card, box, to, send, also, grow, files, voice, say, destroy };
}
