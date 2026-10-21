// The list kept in view with the chat hidden, in the board's own box (room.js) or the web UI's docked one (dock.js):
// however long the words, the box gives way first (it scrolls, never under its one line); while you type with a
// phone's keyboard up, then the board's header and chips step aside, then the chat's bar. The list keeps about a
// heading and two rows, or all the room left over the box's one line where even that is less (a phone on its side).
/* global getComputedStyle */
// The least of the list kept in view: a group's heading and two rows.
export const LIST = 120;
export const STEPS = ["sw-tight", "sw-tighter"];

// `board`: the board, `box`: the field typed in. `set(px)`: how tall the field may be now. `spill()`: how far what holds
// the field runs over the board's foot (that much of it gives way before the list grows). Measured as the board lays
// out now: the field's height and the list's add up to the same however tall the field is, so the answer does not move.
// `steps`: the board's header and the chat's bar may step aside (only while a keyboard is up: it goes down again).
export function keepList(board, box, set, spill = () => 0, steps = true) {
  board.classList.remove(...STEPS);
  const body = board.querySelector(".sw-body");
  if (!body) return;
  const least = Math.ceil(parseFloat(getComputedStyle(box).minHeight)) || 44;
  const room = () => Math.floor(body.getBoundingClientRect().height + box.offsetHeight - spill() - LIST);
  for (const step of steps ? STEPS : []) { if (room() >= least) break; board.classList.add(step); }
  set(Math.max(least, room()));
}
