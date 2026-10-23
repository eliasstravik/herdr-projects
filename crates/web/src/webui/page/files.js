// The files attached in the box you type in (compose.js), as the herdr web UI attaches them in its own chat input: the
// round add button in the lower left picks any files, and a picture pasted into the box or files dropped on it are
// taken too. Each stands as a tile above the message (a picture's thumbnail, else its name), saying where it stands,
// with a cross that takes it out. Each goes to the board (webui/attach.rs), for whoever the words are for, and is kept
// where they can read it; it goes into the message at its start, in the order the files were picked: a picture as
// `[Image 1] `, `[Image 2] ` (numbered per message; one character to the caret, which never stands inside it, and to
// backspace, which takes it out whole, its tile with it), any other file as
// `@<path> `, as in the web UI. What is sent names each picture as `@<path>`, which is how the agent reads it (`expand`,
// docs/image-paste.md). Its limits are the web UI's: 8 MB a file, three on their
// way at a time, and as many at once as its Settings → Chat → Files at once says.
/* global window, document, URL, setTimeout */
import { h, icon, post, webUiSettings } from "./dom.js";

const MOST = 8 * 1024 * 1024, AT_ONCE = 3, LONGEST = 20000;
const PREVIEW = ["image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml"];
export const mention = (path) => `@${path} `;
// A picture in the words: `[Image n]`, its path only in what is sent.
export const chip = (n) => `[Image ${n}]`;
const CHIP = /^\[Image (\d+)\] $/u;
// How many files one pick, paste or drop takes: the web UI's own setting, where it keeps it (20 unless set).
export const perPick = () => { const n = webUiSettings().attachLimit; return Number.isFinite(n) ? Math.min(50, Math.max(1, Math.round(n))) : 20; };
const mb = (n) => `${Math.ceil((n / (1024 * 1024)) * 10) / 10} MB`;
// The web UI's insertLeadingMention: at the start, after a leading /command (it must stay first to be one) and after
// the paths of files picked before it that are still there. Whole or not at all (`length` 0): a cut path names no file.
export function leading(text, m, earlier) {
  let at = /^\/\S+\s*/u.exec(text)?.[0].length ?? 0;
  for (const e of earlier) { const i = text.indexOf(e); if (i >= 0) at = Math.max(at, i + e.length); }
  const before = text.slice(0, at), piece = before && !/\s$/u.test(before) ? ` ${m}` : m;
  if (text.length + piece.length > LONGEST) return { text, at, length: 0 };
  return { text: before + piece + text.slice(at), at, length: piece.length };
}
// Chunked: a screenshot of a few MB is too long for one String.fromCharCode.
function base64(bytes) { let s = ""; for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000)); return window.btoa(s); }

// `whom()`: who the words are for, as chat.js locks them. `hold()`: a message is on its way (a path that comes in
// meanwhile waits, and goes in once it is gone). `changed()`: the files or the words changed. `say(text)`: the note
// over the box (null: none).
export function filesEl({ box, whom, hold = () => false, changed, say }) {
  let list = [], seq = 0, running = 0;
  const queue = [], placed = new Map(), waiting = [];
  const strip = h("div.sw-files", { hidden: true, role: "group", "aria-label": "Attached files" });
  const input = h("input", { type: "file", multiple: true, hidden: true });
  const add = h("button.icon-button.sw-attach", { type: "button", "aria-label": "Attach files", title: "Attach files" }, icon("plus"));
  const busy = () => list.some((f) => f.state === "uploading");
  // The next picture's number: one more than the highest still in the words' files, so each message counts from 1.
  const nextChip = () => 1 + Math.max(0, ...[...placed.values()].map((m) => Number(CHIP.exec(m)?.[1] || 0)));
  const draw = () => {
    strip.hidden = !list.length;
    strip.replaceChildren(...list.map((f) => h(`div.sw-file.is-${f.state}`,
      f.url ? h("img", { src: f.url, alt: f.file.name }) : h("span.sw-file-name", { title: f.file.name }, icon("file"), h("span", f.file.name)),
      h("span.sw-file-state", f.state === "uploading" ? "Uploading" : f.state === "error" ? "Failed" : "Attached"),
      h("button", { type: "button", "aria-label": `Remove ${f.file.name}`, onclick: () => remove(f) }, icon("close")))));
    add.disabled = box.disabled || busy();
    changed();
  };
  // A path is put in while you may be typing: the caret stays where it was in your words. It waits while a message
  // is on its way, while letters are being composed, and while the box holds a /command still being typed (as the
  // web UI's does), so the command stays first.
  const place = () => {
    if (hold() || box.dataset.composing || /^\/\S*$/u.test(box.value)) return;
    waiting.sort((a, b) => a.id - b.id);
    while (waiting.length) {
      const { id, path } = waiting[0], f = list.find((x) => x.id === id);
      if (!f) { waiting.shift(); continue; }
      const m = f.file.type.startsWith("image/") ? `${chip(nextChip())} ` : mention(path), out = leading(box.value, m, [...placed].filter(([p]) => p < id).map(([, x]) => x));
      if (!out.length) break; // no room for it whole: it waits for room, and Send waits with it
      const [a, b] = [box.selectionStart, box.selectionEnd];
      box.value = out.text;
      box.setSelectionRange(a >= out.at ? a + out.length : a, b >= out.at ? b + out.length : b);
      placed.set(id, m); f.state = "ready"; f.path = path; waiting.shift();
    }
    draw();
  };
  const pump = () => {
    while (running < AT_ONCE && queue.length) {
      const f = queue.shift();
      if (!list.includes(f)) continue;
      running += 1;
      send(f.file).then((path) => { if (list.includes(f)) { waiting.push({ id: f.id, path }); place(); } },
        (e) => { if (!list.includes(f)) return; f.state = "error"; say(f.leftOut ? `${e.message} ${f.leftOut}` : e.message); draw(); })
        .finally(() => { running -= 1; pump(); });
    }
  };
  async function send(file) {
    if (file.size > MOST) throw new Error(`Too large to attach: ${file.name || file.type || "file"} (${mb(file.size)}). A file can be up to ${mb(MOST)}.`);
    const out = await post("attach", { name: file.name || "", type: file.type || "", data: base64(new Uint8Array(await file.arrayBuffer())), to: whom() });
    if (!out.ok || !out.path) throw new Error(out.msg || "The file could not be attached.");
    return out.path;
  }
  function take(incoming) {
    if (box.disabled || !incoming.length) return;
    const n = perPick(), files = incoming.slice(0, n);
    const leftOut = files.length < incoming.length ? `Only the first ${files.length} of ${incoming.length} files were taken. Settings → Chat → Files at once sets how many one paste, pick or drop takes.` : null;
    const added = files.map((file) => ({ id: ++seq, file, url: PREVIEW.includes(file.type) ? URL.createObjectURL(file) : "", path: null, state: "uploading", leftOut }));
    list = [...list, ...added];
    say(leftOut);
    draw(); // (chat.js locks who the words are for here, before any of it is sent)
    queue.push(...added);
    pump();
  }
  function remove(f) {
    list = list.filter((x) => x !== f);
    if (f.url) URL.revokeObjectURL(f.url);
    const m = placed.get(f.id);
    placed.delete(f.id);
    if (m) box.value = box.value.includes(m) ? box.value.replace(m, "") : box.value.replace(m.trimEnd(), "");
    draw(); place(); // the room it leaves may be what a finished file waits for
  }
  add.addEventListener("click", () => input.click());
  input.addEventListener("change", () => { const picked = [...(input.files || [])]; input.value = ""; take(picked); });
  // A picture pasted: taken as a file; pasted words go in as words.
  box.addEventListener("paste", (e) => {
    const files = [...(e.clipboardData?.items || [])].filter((i) => i.kind === "file").map((i) => i.getAsFile()).filter(Boolean);
    if (files.length) { e.preventDefault(); take(files); }
  });
  // The pictures' chips whole in the words now: { f, at, end }.
  const chips = () => list.flatMap((f) => {
    const m = placed.get(f.id), c = m?.trimEnd(), at = m && CHIP.test(m) ? box.value.indexOf(c) : -1;
    return at < 0 ? [] : [{ f, at, end: at + c.length }];
  });
  // Backspace just after a picture's chip, or delete just before it, takes the picture out whole, its tile with it.
  box.addEventListener("keydown", (e) => {
    if ((e.key !== "Backspace" && e.key !== "Delete") || e.isComposing || e.altKey || e.ctrlKey || e.metaKey || box.selectionStart !== box.selectionEnd) return;
    const at = box.selectionStart, c = chips().find((x) => (e.key === "Backspace" ? x.end : x.at) === at);
    if (!c) return;
    e.preventDefault();
    box.value = box.value.slice(0, c.at) + box.value.slice(c.end);
    box.setSelectionRange(c.at, c.at);
    placed.delete(c.f.id);
    remove(c.f);
  });
  // The caret never stands inside a chip (a click, an arrow, a tap): it goes past it the way it was going; a selection
  // takes a chip whole or not at all. So no edit cuts one.
  let caret = 0;
  const snap = () => {
    if (document.activeElement !== box) return;
    let [a, b] = [box.selectionStart, box.selectionEnd];
    const one = a === b, back = one && a < caret;
    for (const c of chips()) {
      if (one) { if (a > c.at && a < c.end) a = b = back ? c.at : c.end; continue; }
      if (a > c.at && a < c.end) a = c.at;
      if (b > c.at && b < c.end) b = c.end;
    }
    if (a !== box.selectionStart || b !== box.selectionEnd) box.setSelectionRange(a, b, box.selectionDirection);
    caret = box.selectionEnd;
  };
  for (const [on, name] of [[document, "selectionchange"], [box, "selectionchange"], [box, "select"], [box, "keyup"], [box, "pointerup"], [box, "focus"]]) on.addEventListener(name, snap);
  // A chip changed all the same (cut, typed over in a way the caret did not catch): its picture is no longer in the
  // words, so it goes, its tile with it, and the band says so: never a tile for a picture that is not sent.
  box.addEventListener("input", () => {
    const whole = new Set(chips().map((c) => c.f.id));
    const lost = list.filter((f) => CHIP.test(placed.get(f.id) || "") && !whole.has(f.id));
    for (const f of lost) { const m = placed.get(f.id).trim(); placed.delete(f.id); remove(f); setTimeout(() => say(`${m} was taken out of the message.`), 0); }
    snap();
  });
  box.addEventListener("compositionstart", () => { box.dataset.composing = "1"; });
  box.addEventListener("compositionend", () => { delete box.dataset.composing; setTimeout(place, 0); });
  box.addEventListener("input", () => { if (waiting.length) window.requestAnimationFrame(place); });
  // Dropped on the card: taken, and the card shows it is a place to drop while files are over it.
  const dropOn = (card) => {
    const files = (e) => [...(e.dataTransfer?.types || [])].includes("Files");
    card.addEventListener("dragenter", (e) => { if (!files(e)) return; e.preventDefault(); if (!box.disabled) card.dataset.dragging = ""; });
    card.addEventListener("dragover", (e) => { if (files(e)) e.preventDefault(); });
    card.addEventListener("dragleave", (e) => { if (!card.contains(e.relatedTarget)) delete card.dataset.dragging; });
    card.addEventListener("drop", (e) => { if (!files(e)) return; e.preventDefault(); delete card.dataset.dragging; take([...e.dataTransfer.files]); });
  };
  return {
    strip, input, add, busy, dropOn,
    count: () => list.length,
    place, ids: () => list.map((f) => f.id),
    // The words as they are sent and kept: each picture's chip is `@<path>` again.
    expand: (text) => list.reduce((t, f) => { const m = placed.get(f.id); return m && CHIP.test(m) && f.path ? t.replace(m.trimEnd(), () => `@${f.path}`) : t; }, text),
    // Sent: the tiles of the files sent (`ids`: those there when it went; none: all) go; the files stay where they
    // were kept, for the agent to read. One attached while it was on its way stays, for the next message.
    clear(ids) {
      const gone = ids ? list.filter((f) => ids.includes(f.id)) : list;
      // A sent picture's chip still in the words (they were changed while they went, and stay whole): its path, so
      // what stays names the picture as what was sent did.
      for (const f of gone) { const m = placed.get(f.id); if (m && CHIP.test(m) && f.path) box.value = box.value.replace(m.trimEnd(), () => `@${f.path}`); }
      for (const f of gone) { if (f.url) URL.revokeObjectURL(f.url); placed.delete(f.id); }
      list = list.filter((f) => !gone.includes(f));
      if (!ids) { placed.clear(); waiting.length = 0; queue.length = 0; }
      draw();
    },
    refresh() { add.disabled = box.disabled || busy(); },
  };
}
