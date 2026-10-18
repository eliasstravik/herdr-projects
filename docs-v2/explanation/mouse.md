# Why the board takes the whole mouse

This page tells why the board asks the terminal for all mouse events, and why it draws its own text selection. For what
the mouse does, see [the mouse](../reference/mouse.md).

## A terminal cannot give the wheel alone

A terminal program cannot ask for only the wheel. Without the mouse, Herdr sends the wheel as `↑` and `↓`, and does
not say where the pointer is. Then the board cannot tell a turn on the chat from a turn on the list. Thus the board
asks for the whole mouse.

## Why the board draws its own selection

A program that asks for the mouse gets all of the mouse. Herdr, or a terminal outside Herdr, can then not select text
in the board. Thus the board draws the selection itself, and copies the text with the clipboard code of the terminal
(OSC 52). The result is the same on every terminal.

## Why the wheel does not move the selection while you type

When the text box holds words, the recipient of the words is locked. A turn of the wheel must not send your answer to
a different agent. Thus the wheel on the list moves only the view, and a click selects nothing.

## How it was tested

The mouse was tested on a real Herdr 0.9.3: in a pane, and in the popup. In the popup, the wheel arrives with its
position, as in any pane ([Herdr 0.9.3 facts](../contributing/herdr-0.9.3-facts.md)). Drag-to-copy was not tested in
the popup.

## Related pages

- [The mouse](../reference/mouse.md)
- [Herdr 0.9.3 facts](../contributing/herdr-0.9.3-facts.md)
- [The chat of the board](../reference/board-chat.md)
