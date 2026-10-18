# Why the web board's chat box is built this way

This page tells why the chat box of the web board copies the chat input of the Herdr web UI, and why some parts of it
are different. For the facts, refer to [the web board's chat box](../reference/web-chat-box.md).

## One program for the eye

The box at the foot of the web board looks and works as the chat input of the web UI does. Thus, when you go from the
Board to the chat of an agent and back, the two feel like one program.

## Built again, not loaded

The box does not use the code of the web UI. The input of the web UI is part of its bundled app, and the built files
of that app change with every release. Thus the Board loads nothing from them.

The box is the same thing, built again in the page of the Board (`crates/web/src/webui/page/compose.js` and
`compose.css`). It uses the CSS variables of the web UI and its `DESIGN.md`. Then it was measured against the original
([how](../contributing/web-ui.md#how-the-chat-box-was-measured)). Because it uses the variables of the web UI, it
follows light, dark and every palette of the web UI.

## Where the board's own parts go

The board has things to say that the web UI does not: who gets your words, and if the draft is locked to somebody.
The box shows them in the shape of the web UI:

- The switch over the box tells who gets your words. The box itself does not, except in its hint while it is empty.
- The border of the card tells who gets your words, as the frame of the box does on the terminal board. The border of
  the web UI is grey and changes to its accent on focus. On the Board, the border already tells who, so focus draws a
  ring of that same colour round it.
- **Send + start** uses the shape of the one-tap replies of the web UI.

## Why some controls are not there

The box leaves out each control whose job the Board cannot do. It does not show such a control as a dead control.

- **Stop:** stopping the agent of a task is a button on the open task.
- **The model, the effort and the context ring:** the box of the Board talks to different agents.
- **The menus for `/` and `@`, one-tap replies, the suggestion of the agent, and held messages:** the Board sends your
  words on as they are.
- **Escape that stops the agent:** on the Board, `Esc` goes one step back, as on the terminal board, and the draft
  stays.

## Why a draft is never lost

The board keeps a draft with its receiver, also over a reload. Thus `Esc` does not put a draft away, as it does on the
terminal board, and there is nothing for `Ctrl+Z` to bring back. The terminal way stays on the terminal.

While a message is on its way, the words stay in the box, and a second `Enter` does nothing. Words that
the board could not take also stay in the box.

## Related pages

- [The web board's chat box](../reference/web-chat-box.md)
- [How the web board works](web-ui.md)
- [How the chat box was measured](../contributing/web-ui.md#how-the-chat-box-was-measured)
