# The mouse

The board uses the mouse for three things:

- **The wheel** scrolls the chat when the pointer is over it, and moves the selection anywhere else.
- **A click** on a task selects it. A click on a heading folds or unfolds it.
- **A drag** selects text and copies it when you let go.

Every action is still a key. The keys that scroll the chat without a mouse, `Alt+K` and `Alt+J`, are in
[the keys](keys.md).

## The wheel

- **Over the chat** (the line that says who you talk to, and its box): it scrolls a message that is longer than its
  box, two lines a step, faster when you spin it. It works while you type, and never touches your draft.
- **Anywhere else** it does what `↑` and `↓` do. Over the list it moves the selection one task a step. On the
  whole-task page (`Alt+T`) it scrolls the page, two lines a step, also with a draft in the text box. In the `?` menu it
  moves the pick.
- A question, the first-run screen and a paste on show keep the screen still.

**While the text box holds words**, the wheel over the list only moves the view, never the selection. That is because
who the words go to is locked (the hint line under the box says who: `enter send to HSW-34's agent`), and a turn of the
wheel must not send your answer to another agent. The next key brings the view back to the selected task.

## Clicks

- **A click on a task's row selects it**, like `↑` and `↓`. On the selected row it does nothing more.
- **A click on a heading folds it**, or unfolds it, like `Enter` on it ([list headings](list-headings.md)).
- With words in the box, a click selects nothing and the board says why: `Send or put away what you typed first`.
- A click anywhere else (the header, the chat, the task panel) does nothing.

## Selecting and copying

Drag over text: it is highlighted. Let go: it is copied to the clipboard. A double click selects a word. A click
anywhere clears the highlight.

The board draws this selection itself, the same on every terminal. That is needed because a program that asks for the
mouse gets all of it: Herdr, or a terminal outside Herdr, can no longer select text in the board. The copy goes out as
the terminal's clipboard code (OSC 52), which Herdr passes on with its `copied to clipboard` note. The code is
`crates/board/src/board/run/mouse.rs`.

What this changes for you:

- **Dragging selects and copies as before**, but the board draws the highlight, not Herdr. It selects what is on the
  board's own screen, line by line, box edges included, as Herdr's selection of a pane did.
- **Herdr's own selection settings do not apply in the board's pane.** `copy_on_select = false` (keep the selection
  until `Ctrl+C`) is ignored there: the board always copies when you let go.
- **The terminal's own selection** (the one that crosses Herdr's sidebar and panes) is unchanged. Herdr already holds
  the mouse, so hold the terminal's modifier as anywhere in Herdr: `Shift` and drag in most terminals, `Option` in
  iTerm2, `Fn` in macOS Terminal.
- **Links** (the pull request, the evidence links) are terminal links (OSC 8). Clicking one is the terminal's job
  (`Cmd`+click on a Mac), as before. A plain click on a link reaches the board, which does nothing with it.
- **The wheel over the list moves the selection**, as it did before: Herdr used to turn the wheel into `↑` and `↓`.
- **Some terminals copy nothing on OSC 52** (macOS Terminal; tmux without `set-clipboard on`). There the highlight shows
  but nothing is copied. Herdr's own copy works the same way, so a terminal that copied from Herdr copies from the
  board. Where neither works, use the terminal's own selection with its modifier.

## Why the board takes the whole mouse

Terminals cannot ask for the wheel alone. Without the mouse, Herdr sends the wheel as `↑` and `↓`, without saying where
the pointer is. The board could not tell a turn over the chat from one over the list.

## In the popup

The board works the same in the popup the board key opens (`board.open`: Herdr's own popup). This was tried for real in
a pane of Herdr 0.9.3 and in the popup, where the wheel arrives with its position as in any pane
([checked on Herdr](checked-on-herdr.md)). Drag-to-copy was not tried in the popup.

How the chat grows and where a message starts: [the board](board.md).
