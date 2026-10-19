// The chat at the foot of the board, the terminal board's chat: one button that names who gets your words and flips
// between the two you can talk to (the Coordinator, or the selected task's agent), as space does there; the line on
// where they stand; the conversation (said.js: the newest thing said, the rest above it; full screen, it is the web
// UI's own chat view of them, talk.js, or the board's where the web UI cannot show them) with Hide and Show; and what
// the box you type in (compose.js) does
// with your words: who they are for, kept with them until they are sent, and who a file attached is for; and a voice
// call with whoever it talks to (callbtn.js). Who that
// is, what the box says while empty and what its buttons do come from the board (webui/model/side.rs).
/* global sessionStorage */
import { h, icon, markEl } from "./dom.js";
import { composeEl } from "./compose.js";
import { saidEl } from "./said.js";
import { talkNames, drawTalk } from "./talk.js";
import { callEl } from "./callbtn.js";
import { roomFor } from "./room.js";

const cap = (s) => (s ? s[0].toUpperCase() + s.slice(1) : s);
const DRAFT = "sw-draft";
// What is kept is the draft with who it is for: words that came back after a reload without their recipient would be
// sent to whoever the page shows then.
const TO = "sw-draft-to";
const saved = () => { try { return sessionStorage.getItem(DRAFT) || ""; } catch { return ""; } };
const savedTo = () => { try { const x = JSON.parse(sessionStorage.getItem(TO) || "null"); return x && typeof x === "object" && typeof x.label === "string" ? { id: String(x.id || ""), reply: !!x.reply && !!x.id, scope: String(x.scope || ""), label: x.label, enter: String(x.enter || ""), delegate: String(x.delegate || "") } : null; } catch { return null; } };
const keep = (text, to = null) => { try { sessionStorage.setItem(DRAFT, text); if (text.trim() && to) sessionStorage.setItem(TO, JSON.stringify(to)); else sessionStorage.removeItem(TO); } catch {} };

// Built once, so the text box keeps its words and its cursor while the rest is drawn again.
// `onFull()`: the full screen button was pressed (board.js asks the board where to go).
export function chatEl({ onSend, onPress, onFull }) {
  // `to`: who the words in the box are for, taken when the first of them is typed and kept until they are sent or
  // cleared: { id, reply, label, enter, delegate }. Tapping another task meanwhile changes what you look at, not this.
  let last = null, lastWho = null, to = null, now = null, tone = "text";
  let busy = false, closed = false; // closed: the board was closed, and this box writes nothing more
  let showing = null; // what the conversation shows now, as the board last sent it: { parts, open, raw, toAgent }
  let room = null; // the room over a phone's keyboard (below)
  const go = async (delegate) => {
    const text = box.value;
    if (!text.trim() || busy || files.busy()) return; // a file on its way: its path is not in the words yet
    busy = true; sync();
    // Only what was sent leaves the box: files attached while it is on its way wait (their paths go in after) and stay.
    const sent = files.ids();
    voice.forget(); // sent as it is: a tidied version of dictated words that comes later is not put in
    mine(text);
    const kept = await onSend(text, delegate, to ? { id: to.id, reply: to.reply, scope: to.scope || "" } : null);
    busy = false;
    if (closed) return; // the board closed while it was on its way: a new box has the draft now
    if (!kept) {
      // Words typed after it, while it was on its way, stay; changed inside it, the whole box stays, and says so.
      const edited = box.value !== text && !box.value.startsWith(text);
      box.value = edited ? box.value : box.value.slice(text.length).replace(/^\s+/, ""); files.clear(sent);
      if (edited) say("Sent as it was. Your changes made while it was sending stayed here and were not sent.");
      if (!box.value.trim() && !files.count()) to = null;
      keep(box.value, to); grow();
    }
    files.place(); sync();
  };
  // Your words stand in the conversation the moment they are sent, as on the terminal board, and the line over it says
  // they are on their way: the board's answer, when it comes, is drawn whatever it says (`last` forgotten). Not for
  // somebody the page no longer shows, nor over a program's screen.
  const mine = (text) => {
    const elsewhere = !!to && !!now && (to.id !== now.id || to.reply !== now.reply || (to.scope || "") !== (now.scope || ""));
    if (!showing || !showing.open || showing.raw || elsewhere) return;
    said.set([...showing.parts, { by: "You", text: text.trim(), mine: true }], true, false, showing.parts.length, lastWho !== null);
    said.show(true);
    who.querySelector(".sw-who-status")?.replaceChildren(showing.toAgent ? "Sending…" : "sending…");
    last = null;
  };
  // Tab sends as the terminal's does, and puts an agent on what it adds, where that applies to the words typed.
  // A file attached is for whoever the words are for: attaching one locks that as typing does (sync).
  const whom = () => { const w = to || now; return w ? { id: w.id, reply: w.reply, scope: w.scope || "" } : null; };
  const { el: compose, card, box, to: chip, send, also, grow, files, voice, say, destroy } = composeEl({ onSend: go, onInput: () => { if (closed) return; sync(); keep(box.value, to); }, onTab: () => { if (also.hidden || busy) return false; go(true); return true; }, whom, hold: () => busy, onGrow: () => room?.fit() });
  box.value = saved();
  if (box.value.trim()) to = savedTo(); // a reload brings the words back with who they were for
  // Who you talk to: one button, as space on the terminal board. It names who gets your words; a tap presses the
  // board's own key, which flips to the other (and back with the next tap).
  const talk = h("button.btn.sw-talk", { type: "button", onclick: () => { if (!busy && now && !talk.disabled) onPress("talk"); } });
  const who = h("div.sw-who");
  const hide = h("button.btn.btn-ghost.sw-hide", { type: "button", onclick: () => onPress("chatMessage") });
  const said = saidEl({ onGrow: onFull });
  // The way back while the board's own chat is full screen, where the board's header with its arrow steps aside.
  const back = h("button.icon-button.sw-chat-back", { type: "button", "aria-label": "Back to the board", title: "Back to the board", onclick: () => said.full(false) }, icon("back"));
  const note = h("p.sw-chat-note", { hidden: true });
  // A voice call with whoever the chat talks to: its button beside Hide, its row over the conversation (callbtn.js).
  const phone = callEl();
  let labels = { enter: "", delegate: "" };
  // The first word typed (or file attached) locks who the draft is for, as the box then says; an empty box lets it go.
  function sync() {
    const typing = !!box.value.trim(), holding = typing || files.count() > 0;
    if (!holding) to = null; else if (!to && now) to = { ...now };
    const l = to || labels;
    // Where the page now shows somebody else (another task, another machine's Coordinator), the chip says that the
    // words are not for them: the line above the box names who is shown, the chip who gets the words.
    const moved = !!to && !!now && (to.id !== now.id || to.reply !== now.reply || (to.scope || "") !== (now.scope || ""));
    // The frame's colour says who the box sends to: with a draft locked to somebody the page no longer shows (or
    // before the page knows who it shows, right after a reload), it is the plain one, never the open task's colour.
    card.className = `sw-card sw-b-${to && (moved || !now) ? "text" : tone}`;
    // Who gets the words is said by the switch above; the box says it only when they are for somebody the page no
    // longer shows: then the switch names who is shown, the chip who gets the words.
    chip.hidden = !moved; chip.className = "sw-to is-locked"; chip.textContent = moved ? `To: ${to.label}${to.reply ? " agent" : ""} · not who is shown above` : "";
    send.disabled = !typing || busy || files.busy(); files.refresh(); voice.sync(); send.title = cap(l.enter) || "Send message"; also.hidden = !typing || !l.delegate; also.disabled = busy; also.title = l.delegate ? `${cap(l.delegate)} (Tab)` : "";
  }
  const el = h("footer.sw-chat", { "aria-label": "Chat" }, h("div.sw-chat-head", back, talk, who, hide, phone.btn), note, phone.strip, said.el, compose);
  // Above a phone's keyboard the box stays whole, and the line you type on in view (room.js).
  room = roomFor({ chat: el, said: said.el, box });
  return {
    el,
    typing: () => !!box.value.trim(),
    // The board closed: dictation stops and lets the microphone go, and nothing this box still hears of (an answer, a
    // file, a dictation) changes the kept draft, which the next board's box shows.
    destroy() { closed = true; destroy(); phone.destroy(); room.destroy(); },
    // The conversation full screen (said.js): asked, set, or left.
    full: (on) => said.full(on),
    fill(text) { box.value = text; to = null; sync(); keep(text, to); grow(); box.focus(); },
    update(snap) {
      const c = snap.chat;
      el.hidden = !c;
      if (!c) return;
      // A machine's or a repo's coordinator is drawn in its heading's colour: its name over the box, not what is said (chat.css).
      if (c.tint) el.dataset.tint = c.tint; else delete el.dataset.tint;
      // The switch: the Coordinator (or, with none, a new task), and the selected task's agent (talk.js).
      drawTalk(talk, talkNames(snap));
      // Where they stand, as the terminal's line over the box says it: the mark, the task's name (the switch names the
      // Coordinator), what it is doing. Only what there is: replaceChildren would write a missing part out as "null".
      who.replaceChildren(...[markEl(c.mark.t, c.mark.c), c.toAgent ? h("span.sw-who-name", c.name) : null, h("span.sw-who-status", [c.status, c.progress].filter(Boolean).join("  "))].filter(Boolean));
      who.title = [c.toAgent ? c.name : "", c.status].filter(Boolean).join(": ");
      // Hide and show the latest message: the board's own key for it, so the board remembers which.
      const shown = !!c.open && !!c.text;
      hide.hidden = !c.text; hide.replaceChildren(icon(c.open ? "eyeOff" : "eye"), c.open ? "Hide" : "Show");
      hide.title = c.open ? "Hide the latest message" : "Show the latest message"; hide.setAttribute("aria-pressed", String(!c.open));
      note.hidden = !c.note; note.textContent = c.note;
      phone.update(snap);
      // Who it is (hidden and shown again, it is another), then what they said: an opened chat stays open while only that changes.
      const whose = JSON.stringify([c.toAgent ? snap.list.selected : "", c.coord, !!c.open]);
      const key = JSON.stringify([whose, c.text, c.mine]);
      const fresh = key !== last, same = whose === lastWho;
      if (fresh) { last = key; lastWho = whose; }
      const parts = c.parts && c.parts.length ? c.parts : c.text ? [{ by: "", text: c.text, mine: c.mine, raw: c.raw, tone: c.tone }] : [];
      showing = { parts, open: !!c.open, raw: !!c.raw, toAgent: !!c.toAgent };
      said.set(parts, fresh, !!c.raw, c.parts && c.parts.length ? c.from : 0, same);
      said.show(shown);
      // The box's frame says who gets your words, as on the terminal board: the plain text colour for the Coordinator,
      // the task's status colour for its agent, a heading's colour for its coordinator.
      tone = c.box || "text"; // (drawn by `sync`, below, once who is shown now is known)
      box.placeholder = cap(snap.send.place || "");
      labels = snap.send;
      // Who a draft started now would be for: the task whose agent the chat names, or the Coordinator.
      // (Which machine's Coordinator, too: the view it was typed in is part of who it is for.)
      now = { id: c.toAgent ? snap.list.selected : "", reply: !!c.toAgent, scope: snap.flags?.scope || "", label: c.toAgent ? c.tag || c.name : c.name, enter: snap.send.enter, delegate: snap.send.delegate };
      box.disabled = snap.role !== "drive";
      if (snap.role !== "drive") box.placeholder = "This device can only watch";
      grow(); sync();
      if (to) keep(box.value, to); // locked just now (words that were there before the board answered): kept with them
    },
  };
}
