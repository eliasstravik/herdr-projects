# Herdr 0.9.3 facts: the popup

This page holds the facts about the popup of Herdr 0.9.3 that the board's popup uses. Each entry is one fact, with what
a real test showed. For the list of all facts, see [what was checked on Herdr 0.9.3](herdr-0.9.3-facts.md).

## Popup panes

### Popup panes, and `HERDR_PLUGIN_CONTEXT_JSON` in a popup

- Yes, they work. The JSON has no pane id. It has the workspace, the focused pane and its folder, and the repo root of
  the worktree.

### `agent focus` from inside a popup holds after the popup closes

- Yes.

### A popup pane while another tab shows

- The popup belongs to the tab that it was opened over (`tab_id` in `HERDR_PLUGIN_CONTEXT_JSON`).
- Herdr hides it when another tab or space gets the focus. Its program continues to run. It shows again when its tab
  shows.
- It lies over every pane of that tab, zoomed or not.
- While it exists, shown or hidden, `plugin pane open` of the popup of any plugin answers
  `ui_busy: a popup pane is already open`.
- Since 2026-10-13, the board's popup is this popup. It closes itself when another tab shows (see the entries on the
  popup of Herdr below).

## The popup for the board

### How Herdr's own menu is drawn, and what a plugin gets for it

Read in the source of Herdr at v0.9.3 and at its main branch of 2026-10-13 (`241b99cd`). Its docs, `herdr.dev/llms.txt`
and the preview build of 2026-10-12 say the same.

- The client itself draws the menu, settings, navigator, help and the other dialogs of Herdr, over the whole window
  (`ClientShellOverlay` in `src/client/shell/state.rs`: `GlobalMenu`, `Settings`, `Navigator`, …).
- Each is a box with a border. Nothing behind it is dimmed. Each key and each click is its own.
- No plugin API draws there. A plugin pane has five placements: `overlay`, `split`, `tab`, `zoomed` (ordinary panes)
  and `popup`. Nothing else is for the whole client or the whole session.
- `herdr plugin pane open --help` of 0.9.3 does not list `popup`, but the command accepts it.
- The popup is the nearest thing. There is one popup for the whole session (`popup_pane` in the state of the server),
  drawn over the panes. But the server keeps the tab that the popup opened over (`popup_owner_tab_id` in
  `src/server/headless.rs`). It shows the popup and gives it input only while that tab shows.
- This is not changed on main.

### The popup of Herdr, tried for the board

Tried on 2026-10-13, in a scratch session inside tmux, 140×44, with the plugin linked into it.

- `plugin pane open --placement popup --width 90% --height 90%` drew a window with a frame, titled `Work board`.
- The window was in the middle of the part of the window that the panes have, not over the sidebar and the tab bar.
  It was 102×38 outside (100×36 for the board), in a pane area of 114×43. Without a size, it is half of each dimension.
- Its program gets `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_CONTEXT_JSON` (with `tab_id` and `focused_pane_id`) and what
  `--env` named. It gets no `HERDR_PANE_ID`.
- The layout under it does not change: a pane zoomed before the popup opened was still zoomed after it closed.
- Every key goes to its program. `ctrl+b` then `a`, the board key, arrived as those two keys, and no action ran.
- Clicks outside the popup do nothing. A click on another tab in the tab bar changed nothing.
- The `popup.close` of the socket (there is no command line for it) closes the popup. It ends the program as a closed
  terminal does: the board saved its draft. With no popup open, it answers `popup_not_open`.
- `tab focus` and `workspace focus` from a command still move Herdr under the popup. The popup is then hidden but
  still runs, and it shows again with its tab.
- `events.subscribe` with `tab.focused` sent `tab_focused` with the `tab_id` for each of those moves, also for a move
  to another space.

### The board key with `board.open` set to `popup`, live

Tried in the same session, on the same day.

- The key opened the popup in the middle. The key again (read by the board itself) closed it.
- `Esc` two times (a draft, then nothing left) closed it.
- Words typed before a close were in the box at the next open. This was also true after a close from outside: the
  action invoked through `herdr plugin action invoke` while the popup showed.
- `tab focus` to another tab while the popup showed: the popup was gone at once, with its note. The key opened it over
  the new tab with the same words.
- With a pane zoomed, the popup opened and closed: the pane was still zoomed.
- `Enter` on a task whose pane is in another space: the popup closed, and Herdr was on that pane. The key, pressed
  there, opened the popup on that task.
- These worked: the `?` menu, the remove question and `n`, and the wheel over the list (SGR mouse codes typed into the
  client moved the selection). `Alt+M` also worked: with no saved machine, the board said so.
- In a 60×20 window (the narrow layout of Herdr, panes 60×18), the popup was 54×16. The board had 52×14, with its
  narrow layout.
- In 30×8 (panes 30×6), the popup was 27×5. The board showed its text box and one hint line in 25×3.
- With the popup of another plugin showing, Herdr typed the board key into that popup (`^Ba`). The action invoked from
  a command then got `ui_busy`. The board opened in a tab and said why.
- While the board's popup showed, the `plugin pane open` of the other plugin got `ui_busy`.
- Not tried:
  - sending to a live Coordinator from the popup (it was off in the scratch session);
  - the approve action for the focused pane with a task that waits for approval (tests cover which board asks);
  - a second window on the same session;
  - a terminal that sends keys in the kitty protocol.

### A tab or space changed while the board's popup was still starting

Tried on 2026-10-13, in the same kind of session.

- Herdr tells of `tab_focused` only from the moment that the subscription is taken. The program of the board needs
  about one second to get there.
- A `tab focus` or `workspace focus` in that second was missed. The popup stayed hidden on its tab, and it continued
  to run.
- `tab.get` answers `focused` for a tab. It is true only for the one tab that Herdr shows. It is false for the front
  tab of a space that does not show.
- Thus the board asks `tab.get` when its subscription has started (`subscription_started`). It closes if its tab does
  not show.
- This was tried with the move 0.15, 0.3, 0.6, 0.9 and 1.3 seconds after the key, to another tab and to another space.
  No board stayed running, and no popup showed when the first tab showed again.
- If the move occurs before Herdr runs the action of the key, the popup opens over the tab that shows at that time. It
  stays there.

## Related pages

- [What was checked on Herdr 0.9.3](herdr-0.9.3-facts.md)
- [Herdr 0.9.3 facts: plugin panes and the keyboard](checked-on-herdr-panes.md)
- [Settings](../reference/settings.md)
- [Mouse](../reference/mouse.md)
