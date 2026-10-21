// The chat under the board's list. Its bar says who you talk to and flips it, as space does on the terminal board (the
// Coordinator, or the selected task's agent: two choices of one switch), how they stand, and holds Hide and Full
// screen; under it the call's row while a call rings or is on. The Call button is in the box you type in, beside the mic. Under the bar is the web UI's own chat of whoever it is,
// docked there (dock.js), or, where the web UI has none to show, the board's own: the conversation (said.js) and the
// box (compose.js), with what the box does with your words: who they are for, kept with them until they are sent, and
// who a file attached is for. The bar's grip sets how much of the screen the chat has. Who it is, what the box says
// while empty and what its buttons do come from the board (webui/model/side.rs).
/* global document, window, history, location, sessionStorage, localStorage, getComputedStyle */
import { h, icon, markEl } from "./dom.js";
import { composeEl } from "./compose.js";
import { saidEl } from "./said.js";
import { talkNames } from "./talk.js";
import { callEl } from "./callbtn.js";
import { roomFor } from "./room.js";
import { dockFor } from "./dock.js";

const cap = (s) => (s ? s[0].toUpperCase() + s.slice(1) : s);
const DRAFT = "sw-draft";
// What is kept is the draft with who it is for: words that came back after a reload without their recipient would be
// sent to whoever the page shows then.
const TO = "sw-draft-to";
const saved = () => { try { return sessionStorage.getItem(DRAFT) || ""; } catch { return ""; } };
const savedTo = () => { try { const x = JSON.parse(sessionStorage.getItem(TO) || "null"); return x && typeof x === "object" && typeof x.label === "string" ? { id: String(x.id || ""), reply: !!x.reply && !!x.id, scope: String(x.scope || ""), label: x.label, enter: String(x.enter || ""), delegate: String(x.delegate || "") } : null; } catch { return null; } };
const keep = (text, to = null) => { try { sessionStorage.setItem(DRAFT, text); if (text.trim() && to) sessionStorage.setItem(TO, JSON.stringify(to)); else sessionStorage.removeItem(TO); } catch {} };
// How much of the screen the chat has under the list, set with the bar's grip, kept in this browser: its share of the
// board's height, between a quarter and four fifths.
const SPLIT = "sw-chat-split", LEAST = 0.25, MOST = 0.8, START = 0.46, STEP = 0.05;
const savedSplit = () => { try { const n = Number(localStorage.getItem(SPLIT)); return n >= LEAST && n <= MOST ? n : START; } catch { return START; } };
const keepSplit = (n) => { try { localStorage.setItem(SPLIT, String(n)); } catch {} };
// A screen this wide (the board's part of it) has the list and the chat side by side.
const SIDE = 1000;
// How they stand in a word the mark already says: on a narrow bar the word steps aside.
const PLAIN = /^(idle|working|done|ready|thinking)$/i;

// Built once, so the text box keeps its words and its cursor while the rest is drawn again.
// `onDock()`: who the chat talks to changed, so where their chat is is asked again (board.js: the board's `dock`).
export function chatEl({ onSend, onPress, onDock }) {
  // `to`: who the words in the box are for, taken when the first of them is typed and kept until they are sent or
  // cleared: { id, reply, label, enter, delegate }. Tapping another task meanwhile changes what you look at, not this.
  let lastWho = null, to = null, now = null, tone = "text", status = "", snapNow = null, coordHead = false;
  let busy = false, closed = false; // closed: the board was closed, and this box writes nothing more
  let full = false, open = true, split = savedSplit(), webTyping = false;
  let room = null, dock = null; // the room over a phone's keyboard, and the web UI's chat under the board
  let showing = null; // the conversation as the board last sent it: { parts, open, raw, toAgent }
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
  // Your words stand in the conversation the moment they are sent, as on the terminal board, and the bar says they are
  // on their way. Not for somebody the page no longer shows, nor over a program's screen.
  const mine = (text) => {
    const elsewhere = !!to && !!now && (to.id !== now.id || to.reply !== now.reply || (to.scope || "") !== (now.scope || ""));
    if (!showing || showing.raw || elsewhere) return;
    said.set([...showing.parts, { by: "You", text: text.trim(), mine: true }]);
    state.replaceChildren(showing.toAgent ? "Sending…" : "sending…"); status = ""; // (the board's next answer draws its words again)
  };
  // A file attached is for whoever the words are for: attaching one locks that as typing does (sync).
  const whom = () => { const w = to || now; return w ? { id: w.id, reply: w.reply, scope: w.scope || "" } : null; };
  // A voice call with whoever the chat talks to: its button in the box beside the mic, its row under the bar (callbtn.js).
  // Docked, the button is in the web UI's box, beside its mic (dock.js); with the board's own chat, in the board's box.
  const phone = callEl();
  const { el: compose, card, box, to: chip, send, also, grow, files, voice, say, destroy } = composeEl({ onSend: go, onInput: () => { if (closed) return; sync(); keep(box.value, to); }, onTab: () => { if (also.hidden || busy) return false; go(true); return true; }, whom, hold: () => busy, onGrow: () => room?.fit(), call: phone.btn });
  box.value = saved();
  if (box.value.trim()) to = savedTo(); // a reload brings the words back with who they were for

  // The bar. Who you talk to: one switch of the two you can talk to, as space on the terminal board; a tap on the other
  // presses the board's own key, which flips to them. With nobody else to talk to it only names who it is.
  const who = h("div.sw-who", { role: "group", "aria-label": "Talk to" });
  const state = h("span.sw-who-status");
  const hide = h("button.icon-button.sw-hide", { type: "button", onclick: () => onPress("chatMessage") });
  // Full screen and back: one button in one place, its icon saying which it does now.
  const grow2 = h("button.icon-button.sw-grow", { type: "button", onclick: () => setFull(!full) });
  const grip = h("div.sw-split", { tabIndex: 0, role: "separator", "aria-orientation": "horizontal", "aria-label": "Resize the chat", title: "Drag to give the chat more or less room" });
  const note = h("p.sw-chat-note", { hidden: true });
  const bar = h("div.sw-chatbar", grip, who, state, h("span.sw-chatbar-space"), hide, grow2);
  const said = saidEl();
  const own = h("div.sw-own", said.el, compose);
  let labels = { enter: "", delegate: "" };
  // The first word typed (or file attached) locks who the draft is for, as the box then says; an empty box lets it go.
  function sync() {
    const typing = !!box.value.trim(), holding = typing || files.count() > 0;
    if (!holding) to = null; else if (!to && now) to = { ...now };
    const l = to || labels;
    // Where the page now shows somebody else (another task, another machine's Coordinator), the chip says that the
    // words are not for them: the bar names who is shown, the chip who gets the words.
    const moved = !!to && !!now && (to.id !== now.id || to.reply !== now.reply || (to.scope || "") !== (now.scope || ""));
    // The frame's colour says who the box sends to: with a draft locked to somebody the page no longer shows (or
    // before the page knows who it shows, right after a reload), it is the plain one, never the open task's colour.
    card.className = `sw-card sw-b-${to && (moved || !now) ? "text" : tone}`;
    chip.hidden = !moved; chip.className = "sw-to is-locked"; chip.textContent = moved ? `To: ${to.label}${to.reply ? " agent" : ""} · not who is shown above` : "";
    send.disabled = !typing || busy || files.busy(); files.refresh(); voice.sync(); send.title = cap(l.enter) || "Send message"; also.hidden = !typing || !l.delegate; also.disabled = busy; also.title = l.delegate ? `${cap(l.delegate)} (Tab)` : "";
  }
  const el = h("footer.sw-chat", { "aria-label": "Chat" }, bar, phone.strip, note, own);

  // How the board lays the chat out: docked on the web UI's chat or with its own; the share of the screen it has; full
  // screen; hidden; side by side on a wide screen; stepped aside to the bar while a phone's keyboard is up for the web
  // UI's box (board.css).
  const html = document.documentElement;
  const appHeight = () => { const v = parseFloat(getComputedStyle(html).getPropertyValue("--app-height")); return v > 0 ? v : window.innerHeight; };
  function layout() {
    const board = el.closest(".sw-board");
    if (!board) return;
    const docked = !!dock?.on();
    // (What the board may take: the window beside the web UI's sidebar, not the board's own width, which side by side is less.)
    const side = !full && open && window.innerWidth - (parseFloat(board.style.left) || 0) >= SIDE;
    board.classList.toggle("is-docked", docked);
    board.classList.toggle("sw-chat-full", full);
    board.classList.toggle("sw-chat-hidden", !open && !full);
    board.classList.toggle("is-side", side);
    board.classList.toggle("sw-web-typing", webTyping);
    own.hidden = docked;
    if (!docked && voice.el.nextSibling !== phone.btn) voice.el.after(phone.btn); // back from the web UI's box
    said.show(!docked && (open || full));
    const sized = !full && !webTyping && !side;
    // Docked, the board ends where the web UI's chat begins: the chat's share of the screen under it, or with the chat
    // hidden only the web UI's box. With its own chat, the conversation's height is that share.
    const H = appHeight(), boxH = parseFloat(html.style.getPropertyValue("--sw-composer-h")) || 0;
    if (docked && sized) board.style.setProperty("--sw-top-h", `${Math.round(open ? H - Math.max(boxH + 140, split * H) : H - boxH)}px`);
    else board.style.removeProperty("--sw-top-h");
    board.style.setProperty("--sw-chat-share", `${Math.round(split * H)}px`);
    grip.hidden = !sized || !open;
    hide.replaceChildren(icon(open ? "down" : "up"));
    hide.title = open ? "Hide the chat" : "Show the chat"; hide.setAttribute("aria-label", hide.title); hide.setAttribute("aria-pressed", String(!open));
    // Full screen, the same button in the same place goes back (Hide has nothing to hide then).
    hide.hidden = full;
    grow2.replaceChildren(icon(full ? "shrink" : "expand"));
    grow2.title = full ? "Exit full screen" : "Full screen"; grow2.setAttribute("aria-label", grow2.title); grow2.setAttribute("aria-pressed", String(full));
    dock?.fit();
  }
  // Full screen is a step in the browser's history too, so a phone's back gesture (or Back) leaves it, as it left the
  // web UI's own full screen chat before. Left any other way (its button, Esc), that step is taken back.
  function setFull(on) {
    if (full === on) return;
    full = on; layout();
    try {
      if (on) history.pushState({ ...(history.state || {}), swFull: 1 }, "", location.href);
      else if (history.state?.swFull) history.back();
    } catch {}
  }
  const popped = () => { if (full && !history.state?.swFull) { full = false; layout(); } };
  window.addEventListener("popstate", popped);
  // Dragging the grip moves where the list ends and the chat begins; the keys move it a step.
  grip.addEventListener("pointerdown", (e) => {
    if (e.button > 0) return;
    e.preventDefault(); grip.setPointerCapture?.(e.pointerId);
    const board = el.closest(".sw-board"), top = board.getBoundingClientRect().top, H = appHeight();
    const move = (m) => { split = Math.min(MOST, Math.max(LEAST, (top + H - m.clientY) / H)); layout(); };
    const end = () => { for (const [n, fn] of [["pointermove", move], ["pointerup", end], ["pointercancel", end]]) grip.removeEventListener(n, fn); keepSplit(split); };
    grip.addEventListener("pointermove", move); grip.addEventListener("pointerup", end); grip.addEventListener("pointercancel", end);
  });
  grip.addEventListener("keydown", (e) => {
    if (e.key === "ArrowUp") split = Math.min(MOST, split + STEP); else if (e.key === "ArrowDown") split = Math.max(LEAST, split - STEP); else if (e.key === "Home") split = START; else return;
    e.preventDefault(); keepSplit(split); layout();
  });
  window.addEventListener("resize", layout);

  // The bar's switch: the two you can talk to, the one you talk to now pressed. Each with how it stands where the board
  // says so: the one you talk to by the chat's own mark, a task's agent by its row's.
  function drawWho(snap, c) {
    const { is, other } = talkNames(snap);
    const row = snap.list?.groups?.flatMap((g) => g.rows).find((r) => r.id === snap.list.selected);
    const agentMark = row ? markEl(row.mark, row.color, row.markDim) : null;
    const choice = (name, pressed, mark) => h(`button.sw-who-pick${pressed ? ".is-on" : ""}`, { type: "button", "aria-pressed": String(pressed), title: pressed ? `Talking to ${name}` : `Talk to ${name} instead`, onclick: () => { if (!pressed && !busy && now) onPress("talk"); } }, mark, h("span.sw-who-name", name));
    const mark = markEl(c.mark.t, c.mark.c);
    if (!other) { who.className = "sw-who is-one"; who.replaceChildren(h("span.sw-who-pick.is-on", mark, h("span.sw-who-name", is))); return; }
    // The Coordinator first, the task's agent (or a heading's coordinator) second, whichever you talk to now.
    const [coord, agent] = c.toAgent ? [other, is] : [is, other];
    who.className = "sw-who segmented";
    who.replaceChildren(choice(coord, !c.toAgent, c.toAgent ? null : mark), choice(agent, !!c.toAgent, c.toAgent ? mark : agentMark));
  }
  // Where the web UI's chat of them is asked again when who it is changes, and while the board's own stands in, when
  // how they stand changes (a Coordinator that has just started).
  let asked = "";
  const ask = (whose) => { if (whose === asked) return; asked = whose; onDock(); };

  dock = dockFor({ boardOf: () => el.closest(".sw-board"), onChange: () => layout(), onTyping: (on) => { webTyping = on; layout(); }, guest: phone.btn,
    // A coordinator's words in the web UI's box (the Coordinator's, a machine's or a repo's) go through the board; a
    // task's agent's as the web UI sends them.
    route: () => (snapNow?.chat && now && (!now.reply || coordHead) && snapNow.role === "drive" ? { send: (text) => onSend(text, false, { id: now.id, reply: now.reply, scope: now.scope || "" }) } : null) });
  // Above a phone's keyboard the board's own box stays whole, and the line you type on in view (room.js).
  room = roomFor({ chat: el, said: said.el, box });
  return {
    el,
    typing: () => !!box.value.trim(),
    // The board closed: dictation stops and lets the microphone go, the web UI goes back to what it showed, and nothing
    // this box still hears of (an answer, a file, a dictation) changes the kept draft, which the next board's box shows.
    destroy() { closed = true; destroy(); phone.destroy(); room.destroy(); dock.destroy(); window.removeEventListener("resize", layout); window.removeEventListener("popstate", popped); },
    // The chat full screen: asked, set, or left.
    full: (on) => { if (on === undefined) return full; setFull(!!on); return full; },
    // Where the web UI's chat of whoever it talks to is (null: none, the board's own chat).
    native(go) { dock.show(go || null); layout(); },
    fill(text) { box.value = text; to = null; sync(); keep(text, to); grow(); box.focus(); },
    update(snap) {
      const c = snap.chat;
      snapNow = snap;
      el.hidden = !c;
      if (!c) return;
      // A machine's or a repo's coordinator is drawn in its own colour: its name in the bar (chat.css).
      if (c.tint) el.dataset.tint = c.tint; else delete el.dataset.tint;
      drawWho(snap, c);
      // Talking to a machine's or a repo's coordinator (on its heading), not to a task's agent.
      coordHead = !!c.toAgent && !!c.head && !(snap.list?.selected && snap.flags?.onTask !== false && c.task);
      // How they stand, as the terminal's line over the box says it, with the check's progress.
      const words = cap([c.status, c.progress].filter(Boolean).join(" · "));
      if (words !== status) { status = words; state.replaceChildren(words); state.title = words; }
      state.classList.toggle("is-plain", PLAIN.test(c.status || "") && !c.progress);
      open = !!c.open;
      note.hidden = !c.note; note.textContent = c.note;
      phone.update(snap);
      // Who it is: a new one asks where their chat is; while the board's own stands in, so does a change of how they stand.
      const whose = JSON.stringify([c.toAgent ? snap.list.selected : "", c.coord, c.name, snap.flags?.scope || ""]);
      if (whose !== lastWho) { lastWho = whose; asked = ""; }
      ask(dock.on() ? whose : JSON.stringify([whose, c.status]));
      const parts = c.parts && c.parts.length ? c.parts : c.text ? [{ by: "", text: c.text, mine: c.mine, raw: c.raw, tone: c.tone }] : [];
      showing = { parts, raw: !!c.raw, toAgent: !!c.toAgent };
      said.set(parts);
      // The box's frame says who gets your words, as on the terminal board: the plain text colour for the Coordinator,
      // the task's status colour for its agent, a heading's coordinator's own colour for it.
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
      layout();
    },
  };
}
