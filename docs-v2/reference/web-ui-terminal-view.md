# The terminal board in the web UI's terminal view

This page gives the facts about the terminal board when the Herdr web UI shows it in its terminal view. Use it to find
how to type a key there, on a desktop and on a phone.

The terminal board is an ordinary pane. Thus the web UI shows it in its terminal view as it shows other panes, on this
machine and on its other PCs. For why the board needed changes to work there, refer to
[the explanation](../explanation/web-ui.md#the-terminal-board-in-the-terminal-view).

## What the board does in the terminal view

- **The view keeps the terminal.** A view that somebody looks through keeps the terminal, and the board draws itself
  for that view ([board size](../explanation/board-size.md)).
- **`Alt` keys and `Shift+Enter` work.** The terminal of the web UI sends the old byte forms for them, and the board
  reads those forms too ([keys](keys.md)).
- **The phone's input line** sends an `Enter` of its own after what you type. While a view of the pane is open, the
  board ignores that `Enter` after `?`, `y`, `n` or space. Thus `?` opens the menu, and the menu stays open.

## Keys

| The key | In a desktop browser | On a phone |
|---|---|---|
| Letters, `Enter`, `Tab`, `Esc`, arrows | Type them. | Use the input line and the key bar (`Esc` `Tab` `Enter` and the arrows). |
| `Ctrl` keys | Type them. A browser keeps a few keys for itself (`Ctrl+N`, `Ctrl+T`, `Ctrl+W` in Chrome): pick that line in the `?` menu. | Push `Ctrl` on the key bar, then the letter with direct typing on (the keyboard button). Or use the `?` menu. |
| `Alt` keys | Type them. On a Mac, Option types a character in place of the key: pick the line in the `?` menu. | Push `Alt` on the key bar, then the letter with direct typing on. Or use the `?` menu. |
| `?` | Type it. | Type it into the input line and send it. |

## Going into an agent

- **A task on this machine:** the Herdr window goes to the agent. The board tells you to pick the agent in the sidebar
  of the web UI.
- **A task on another machine:** the board does not change its terminal into the terminal of that agent while the web
  UI shows it. The board tells you where the agent is in the sidebar.
- From a browser, use **Open this task's agent** on the web board ([the web board](web-ui.md)).

## Related pages

- [The web board](web-ui.md)
- [How the web board works](../explanation/web-ui.md)
- [Keys](keys.md)
- [Board size](../explanation/board-size.md)
- [Testing the web board](../contributing/web-ui.md)
