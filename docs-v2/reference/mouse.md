# The mouse

This page lists what the mouse does on the board: the wheel, clicks, and selecting and copying text. For the reasons,
read [why the board takes the whole mouse](../explanation/mouse.md).

The board uses the mouse for three things:

- **The wheel** scrolls the chat when the pointer is on the chat. In all other places, it moves the selection.
- **A click** on a task selects the task. A click on a heading folds or unfolds the heading.
- **A drag** selects text. When you release the button, the board copies the text.

You can also do every action with a key. The keys that scroll the chat without a mouse are `Alt+K` and `Alt+J`
([keys](keys.md)).

## The wheel

| Where the pointer is | What the wheel does |
|---|---|
| the chat (the line that names who you talk to, and its box) | It scrolls a message that is longer than its box, two lines for each step. It scrolls faster when you turn the wheel faster. It works while you type, and it does not change your draft. |
| the list | It does what `↑` and `↓` do: it moves the selection one task for each step. |
| the whole-task page (`Alt+T`) | It scrolls the page, two lines for each step, also when the text box holds a draft. |
| the `?` menu | It moves the pick. |
| a question, the first-run screen, a paste on show | Nothing. The screen does not move. |

**While the text box holds words**, the wheel on the list moves only the view, not the selection. The hint line under
the box names the recipient of the words (`enter send to HSW-34's agent`). The next key moves the view back to the
selected task.

## Clicks

| Where you click | Result |
|---|---|
| the row of a task | It selects the task, as `↑` and `↓` do. On the selected row, it does nothing more. |
| a heading | It folds or unfolds the heading, as `Enter` on the heading does ([list headings](list-headings.md)). |
| any place, while the box holds words | It selects nothing. The board shows `Send or put away what you typed first`. |
| any other place (the header, the chat, the task panel) | Nothing. |

## Selecting and copying

- Drag on text to highlight it.
- Release the button to copy the highlighted text to the clipboard.
- Double-click a word to select it.
- Click any place to remove the highlight.

The board draws this selection itself, the same on every terminal. The copy goes to the clipboard as the clipboard
code of the terminal (OSC 52). Herdr passes the code on, and shows its `copied to clipboard` note.

What this means for you:

- **A drag selects and copies text, as before**, but the board draws the highlight, not Herdr. It selects what is on the
  screen of the board, line by line, with the box edges, as Herdr did for a pane.
- **Herdr's own selection settings do not apply in the pane of the board.** The board ignores
  `copy_on_select = false` (keep the selection until `Ctrl+C`). The board always copies when you release the button.
- **The selection of the terminal** (the selection that crosses Herdr's sidebar and panes) does not change.
  Herdr already holds the mouse, so hold the modifier of the terminal while you drag, as in all of Herdr: `Shift` in
  most terminals, `Option` in iTerm2, `Fn` in macOS Terminal.
- **Links** (the pull request, the evidence links) are terminal links (OSC 8). The terminal opens a link when you click
  it (`Cmd`+click on a Mac), as before. A plain click on a link goes to the board, and the board does nothing with it.
- **The wheel on the list moves the selection**, as before. Before, Herdr changed the wheel into `↑` and `↓`.
- **Some terminals copy nothing on OSC 52**: macOS Terminal, and tmux without `set-clipboard on`. There, the highlight
  shows, but nothing goes to the clipboard. Herdr's own copy works in the same way. Thus a terminal that copied from
  Herdr also copies from the board. If neither works, use the selection of the terminal with its modifier.

## In the popup

The mouse works the same in the popup that the board key opens (`board.open`: Herdr's own popup). Drag-to-copy was not
tested in the popup ([why the board takes the whole mouse](../explanation/mouse.md#how-it-was-tested)).

## For contributors

The code of the mouse is in [`crates/board/src/board/run/mouse.rs`](../../crates/board/src/board/run/mouse.rs).

## Related pages

- [Why the board takes the whole mouse](../explanation/mouse.md)
- [Keys](keys.md)
- [List headings](list-headings.md)
- [Fold list headings](../how-to/fold-list-headings.md)
- [The chat of the board](board-chat.md) and [the messages in the chat](board-chat-messages.md): how the chat grows and where a message starts
