# Herdr 0.9.3 facts: plugin panes and the keyboard

This page holds the facts about plugin panes in Herdr 0.9.3 other than the popup, and about which pane gets the
keyboard. Each entry is one fact, with what a real test showed. For the list of all facts, see
[what was checked on Herdr 0.9.3](herdr-0.9.3-facts.md).

## Placements other than the popup

### A plugin pane opened with `--placement tab`

- It is a new tab in the focused space, which holds one ordinary pane.
- Its program gets `HERDR_PANE_ID`, `HERDR_TAB_ID` and `HERDR_PLUGIN_ID`. `HERDR_PLUGIN_CONTEXT_JSON` still names the
  pane that you came from.
- The answer carries `plugin_pane.pane` (pane, tab, space).
- The tab stays when you go elsewhere. The popups of other plugins open beside it. `tab rename` names it. Its tab
  closes when the program ends.
- `plugin pane focus <pane>` from another space did not move the client there. `workspace focus` then `tab focus`
  does.

### A plugin pane opened with `--placement overlay`

Tried on 2026-10-13, in a scratch session inside tmux.

- It is an ordinary pane with an id. It is split off the tab that shows, and zoomed. It is drawn inside a frame with
  the title of the pane.
- It has the whole size of the tab, less the frame. 134×44 gave the board 132×42.
- Herdr refuses `--width` and `--height` (`width and height are only supported when placement is popup`).
- Its program gets `HERDR_PANE_ID`, `HERDR_TAB_ID`, `HERDR_PLUGIN_ID` and what `--env` named. It also gets
  `HERDR_PLUGIN_CONTEXT_JSON` with the pane that you came from (`focused_pane_id`) and its tab.
- The own keys of Herdr still work in it. The board key ran its action with the overlay focused. (In the floating
  popup, Herdr gave the board `ctrl+b` and `a` as typed keys, and ran nothing.)
- `plugin pane close <id>` closes it and ends its program, as a closed terminal does. The board saved its draft.
- It is in `pane list`. `pane layout` reports it as the zoomed, focused pane.
- It opened while the floating popup of another plugin was open, both when that popup was hidden behind another tab
  and when it showed. It opens under that popup. That popup also opened over it. There was no `ui_busy` in either
  direction.
- If you leave it over a tab, it is hidden with that tab and continues to run.
- The zoom key of Herdr (`prefix+z`) unzooms it into a split pane.
- A pane that was zoomed before it opened is not zoomed after it closes.
- Until 2026-10-13, the board key opened this pane for `board.open` `popup`. It does not do so now.

### The board key with `board.open` set to `popup`, while `popup` meant the overlay, live

Tried in the same session, on the same day. The floating popup of a second plugin was the popup of "another plugin".
This entry is kept for what it tells about the overlay.

- The key opened the overlay over the focused tab. The key again closed it.
- `Esc` two times (a draft, then nothing left) closed it.
- Words typed before a close were in the box at the next open.
- Left over tab 1 and pressed in tab 2: it closed there and opened here. The same occurred when it was left in one
  space and pressed in another.
- Pressed in the pane that a task is linked to: the popup opened on that task.
- `Enter` on a task from another space, and from the tab that the popup lay over: the popup closed. Herdr was on the
  pane of that task, not zoomed. The pane was at its shell, so Herdr went to its space and tab. (`agent focus` on a
  running agent from a popup was checked earlier: see [the popup](checked-on-herdr-popup.md).)
- With a board running in a tab, both boards ran, each with its own note and its own draft, and one keeper.
- In the popup, these worked:
  - typing a task and `Enter`;
  - the remove question and `n`;
  - the `?` menu and `Esc` out of it;
  - the wheel over the list (SGR mouse codes typed into the client moved the selection);
  - the tasks of other machines (`Alt+M`, a saved machine);
  - the layouts at 56×18 and 56×8.
- While the floating popup of another plugin showed, the board key never reached the plugin: Herdr typed it into that
  popup. The action invoked through `herdr plugin action invoke` then opened the board under it.
- Not tried:
  - sending to a live Coordinator from the popup (it was off in the scratch session, and the words became a task);
  - Herdr answering `ui_busy` to the overlay, which it never did (a test with the fake Herdr covers what the board
    says then).

## The board key in a tab, and the keyboard

### The board key with `board.open` left at `tab`, live

Tried in the same session.

- Pressed in the pane of a shell: the tab of the board opened, and its note named that pane.
- Pressed on the board: Herdr was back on that pane.
- Pressed again: the board.
- With nothing to go back to, the message line of the board said so within one second.

### Giving one pane the keyboard by its id

- No command does it. `pane focus` only goes to a neighbour. `workspace focus` and `tab focus` leave the keyboard on
  the pane of the tab that had it.
- `pane zoom <id> --on` focuses that pane, also when another pane of the tab was zoomed. `--off` after it leaves the
  focus there.
- The board key uses this to come back to a board that shares its tab with an agent.

### A second viewer on the terminal of a pane (`herdr terminal attach`)

- The viewer sets the size of the terminal. Herdr then shows a part of it, from the top left.
- `pane layout` still gives the size that Herdr shows. Thus the board draws inside that size and tells what holds it.
- `terminal session control --takeover`, left at once, disconnects the other viewer. Herdr then sizes the terminal to
  the pane again. The board does this by itself (the setting `ownTerminal`).

  **Note:** `ownTerminal` is now only an old key. The board reads it only to know a config file of the old shape
  (`OLD_KEYS` in `crates/core/src/store/settings/migrate.rs`), and it moves it to no new setting. The board now takes
  the terminal back with no setting for it (`crates/board/src/board/update/size.rs`), unless somebody looks through
  the view that holds it.
- Nothing else gives the size back while the viewer is attached: not a client resize, not zoom, and not `control`
  without `--takeover`.
- This does not apply in the popup, because the popup has no pane id.

## Related pages

- [What was checked on Herdr 0.9.3](herdr-0.9.3-facts.md)
- [Herdr 0.9.3 facts: the popup](checked-on-herdr-popup.md)
- [Herdr 0.9.3 facts: terminal sizes, tabs and the Coordinator](checked-on-herdr-tabs.md)
- [Settings](../reference/settings.md)
- [Tabs](../reference/tabs.md)
