// The room the chat has above a phone's keyboard. While you type with the keyboard up, the board's header, its chips
// and the list step aside (as for the chat full screen, chat.css), the conversation takes what is left over the box and
// steps aside under its least, and the box you type in is never taller than the room left: its card stays whole above
// the keyboard and the line with the caret stays in view, however long the words get. On an iPhone it also leaves out
// the strip iOS floats over the foot of the page while its keyboard is up (its arrows and Done), which the visible
// window does not leave out.
/* global window, document, navigator, getComputedStyle, requestAnimationFrame, ResizeObserver, MutationObserver,
   setTimeout */

// The least the conversation is shown at: its Earlier and Later buttons, one over the other (two thumbs and its
// padding). How much smaller than the window the visible part must be for a keyboard to be up. The strip iOS 26 floats
// over the page: its top stands 73 px over the foot of the visible window (measured on an iPhone's screenshot), so the
// box, 8 px over that foot, keeps 72 more and stands clear of it.
const LEAST = 104, KEYBOARD = 120, IOS_BAR = 72;
const ios = () => /iP(hone|ad|od)/.test(navigator.userAgent) || (/Macintosh/.test(navigator.userAgent) && navigator.maxTouchPoints > 1);

// `chat`: the chat at the board's foot, `said`: the conversation's box, `box`: the field you type in.
export function roomFor({ chat, said, box }) {
  const html = document.documentElement, vv = window.visualViewport;
  let queued = false, height = 0, words = "";
  // The web UI marks the page while a keyboard is up (its shell, as it writes --app-height); elsewhere the visible
  // window tells, unless it is only zoomed in.
  const keyboard = () => html.hasAttribute("data-keyboard") || (!!vv && Math.abs(vv.scale - 1) < 0.01 && window.innerHeight - vv.height > KEYBOARD);
  const px = (v) => parseFloat(v) || 0;
  // Where the caret's line starts in the field's words: the words up to it, laid out alike out of sight.
  const caretTop = () => {
    const cs = getComputedStyle(box), copy = document.createElement("div"), at = document.createElement("span");
    copy.style.cssText = `position:absolute;top:0;left:-9999px;visibility:hidden;box-sizing:border-box;white-space:pre-wrap;overflow-wrap:anywhere;width:${box.clientWidth}px;padding:${cs.padding};font:${cs.font};letter-spacing:${cs.letterSpacing};line-height:${cs.lineHeight}`;
    copy.textContent = box.value.slice(0, box.selectionEnd); at.textContent = "\u200b"; copy.append(at);
    document.body.append(copy); const y = at.offsetTop; copy.remove();
    return y;
  };
  // The field scrolls to the caret's line where it is out of view.
  const showCaret = () => {
    if (box.scrollHeight <= box.clientHeight) return;
    if (box.selectionEnd >= box.value.length) { box.scrollTop = box.scrollHeight; return; }
    const cs = getComputedStyle(box), line = px(cs.lineHeight) || 24, y = caretTop();
    if (y - px(cs.paddingTop) < box.scrollTop) box.scrollTop = y - px(cs.paddingTop);
    else if (y + line + px(cs.paddingBottom) > box.scrollTop + box.clientHeight) box.scrollTop = y + line + px(cs.paddingBottom) - box.clientHeight;
  };
  const fit = () => {
    queued = false;
    const board = chat.closest(".sw-board");
    if (!board) return;
    const mine = document.activeElement === box, on = mine && keyboard();
    // Measured again from the start: the field is as tall as its words for a moment, so where it was scrolled to is
    // put back after (or it would end scrolled up from the caret's line).
    const top = box.scrollTop;
    board.classList.toggle("sw-typing", on); board.classList.remove("sw-cramped");
    said.classList.remove("is-squeezed"); box.style.removeProperty("--sw-room");
    if (on) {
      // What of the board is under the keyboard (where it was not made as short as what is over it), and the strip.
      const bottom = vv ? vv.offsetTop + vv.height : window.innerHeight;
      board.style.setProperty("--sw-kb-cover", `${Math.max(0, Math.round(board.getBoundingClientRect().bottom - bottom)) + (ios() ? IOS_BAR : 0)}px`);
      // The conversation has what the box leaves; under its least it steps aside for the words.
      if (!said.hidden && said.getBoundingClientRect().height < LEAST) said.classList.add("is-squeezed");
    } else board.style.removeProperty("--sw-kb-cover");
    // Still too tall for the board (a short window, or a keyboard the page was not told of): the field is as tall as
    // the room left, and scrolls (never under its one line).
    const card = box.closest(".sw-card") || box;
    const over = () => card.getBoundingClientRect().bottom - (Math.min(chat.getBoundingClientRect().bottom, board.getBoundingClientRect().bottom) - px(getComputedStyle(chat).paddingBottom));
    const cap = () => { const o = over(); if (o > 0.5) box.style.setProperty("--sw-room", `${Math.max(px(getComputedStyle(box).minHeight), Math.floor(box.offsetHeight - o))}px`); };
    cap();
    // Not even one line fits over the keyboard (a phone on its side): all but the box steps aside, the files' tiles
    // too (they stay attached: their paths are in the words).
    if (on && over() > 0.5) { board.classList.add("sw-cramped"); box.style.removeProperty("--sw-room"); cap(); }
    box.scrollTop = top;
    // The caret's line in view where the field changed under it: its height, or words put in (a file's path, dictation).
    if (!mine) height = 0; else if (box.clientHeight !== height || box.value !== words) { height = box.clientHeight; showCaret(); }
    words = box.value;
  };
  const soon = () => { if (!queued) { queued = true; requestAnimationFrame(fit); } };
  // The keyboard coming and going, the board's size, and what stands over the box (a band, the dictation's pill).
  vv?.addEventListener("resize", soon); vv?.addEventListener("scroll", soon); window.addEventListener("resize", soon);
  // Leaving waits a moment, so what a tap that took the focus lands on is still where it was tapped.
  box.addEventListener("focus", soon); box.addEventListener("blur", () => setTimeout(soon));
  const marks = typeof MutationObserver === "function" ? new MutationObserver(soon) : null;
  marks?.observe(html, { attributes: true, attributeFilter: ["data-keyboard", "style"] });
  const sizes = typeof ResizeObserver === "function" ? new ResizeObserver(soon) : null;
  sizes?.observe(chat);
  const composer = box.closest(".sw-composer");
  if (composer) sizes?.observe(composer);
  return {
    // The words changed (or who they are for): measured again at once, so the field never shows too tall.
    fit,
    destroy() {
      vv?.removeEventListener("resize", soon); vv?.removeEventListener("scroll", soon); window.removeEventListener("resize", soon);
      marks?.disconnect(); sizes?.disconnect();
    },
  };
}
