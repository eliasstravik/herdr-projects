# The board key, the board tab and the popup

This page gives the facts about where the board opens: going into an agent, the board key, the tab of the board, and
the board as a popup.

## Going into an agent

Going into an agent never ends the board. `Enter` on a task, with nothing typed, moves Herdr to the real pane of the
agent of that task. The board continues to run.

- `Enter` does this from the row, with the chat hidden, or while the chat talks to that agent.
- When you come back, or press the board key, the board is as you left it: the same repo, the same selected task, the
  same filter and the same draft.
- While you are away, the selection stays on that task. This is also true if its agent stops or finishes, and the task
  moves to another group or does not match your search any more. Its row stays until you select a different task.
- Outside a Herdr client (plain SSH), the terminal belongs to the agent until you detach. Then the board is back.

## The board key and the board tab

The board key opens the board in a Herdr tab of its own, named `board`, in the space that you are in. After that, the
key goes to that running board from anywhere. It does not open a second board.

- The key also finds a board that you started yourself in a pane (`herdr-projects` typed there). The key puts the
  keyboard on that board, also when the pane of an agent is split next to it in the same tab.
- **On the board itself, the key takes you back** to the pane that you came from. Thus the key is a way there and a way
  back.
- If that pane is closed, or if you started the board by hand, the message line of the board says that it is the board
  already.
- `Ctrl+C` twice closes the board and its tab.
- **The Coordinator continues to run** after the board closes. While it runs, the line that asks for the second
  `Ctrl+C` says so. The Coordinator is not the Coordinator of this board but of the machine. It is the same one that
  the popup, the web board and `coordinator tell` talk to. It can be at work on what you asked.
- `herdr-projects coordinator stop` stops the Coordinator ([the Coordinator](../explanation/coordinator.md)).

### The name of the tab

A board that you start with `herdr-projects` names its tab `herdr-projects` while the board is open. Thus you can tell
it apart from your other tabs.

- The board renames only that one tab.
- When the board closes (also when it closes to take you into an agent), the tab gets back the name that it had.
- If you gave the tab a name yourself while the board was open, the tab keeps that name.
- A board that was killed leaves the name. The next board that opens and closes in that tab puts back the old name.
- Nothing is renamed in these cases:
  - by the popup (it has no tab of its own);
  - in the tab that the board key opens (that tab is named `board`);
  - outside Herdr;
  - in a tmux inside the pane;
  - by a command that opens no board (`add`, `list`, `show` and the others).
- The code is `name_board_tab` in `crates/board/src/board/run/place.rs`.

## The board as a popup

`herdr-projects set board.open popup` makes the board key open the board as a popup, and close it again
([settings](settings.md)). `unset board.open` brings back the tab.

The popup is the own popup of Herdr: a framed window with the name of the board, in the middle of the screen of Herdr.
It is nine tenths of the width and the height of that screen.

- The popup is not a pane and not a tab. Herdr draws it over what is there and changes nothing under it. When it
  closes, the panes, their sizes and a zoomed pane are as they were.
- While the popup shows, it has the keyboard and the mouse, as the own menu of Herdr does. Nothing behind it takes a
  key or a click.
- **The board key closes the popup.** These also close it:
  - `Esc`, when there is nothing left to step back from. As on every board, these come first: a draft, the whole task,
    the filter, the archive, and a chat message that shows.
  - `Ctrl+C` twice.
- When the popup opens again, what you typed, the selected task and the filter are there.
- **`Enter` on a task closes the popup and opens the agent of that task.** The first three times, it shows for a moment
  how to get back.
- **The popup is open or closed, never left behind.** Nothing in the own window of Herdr can change the tab or the
  space under it.
- If Herdr shows another tab all the same (a command moved it there, or a second window on the same session did), the
  popup closes at that moment. It saves what you typed. The board key opens the popup there.
- **The popup opens on the task that you looked at.** If you press the key in the pane of the agent or the reviewer of
  a task, the popup selects that task. It does this also if the filter from the last time hides the task.
- In that case, a view of one other machine gives way to the tasks of this machine. Thus the task shows, and the chat
  talks to this machine.
- If you press the key anywhere else, the popup opens where it was.

### Words in the popup stay with their repo

- The popup opens in the repo of the pane that you are in. It sends what you type from there.
- If a draft from the last time waits, the popup opens in the repo that the draft was typed for, and says so. Thus
  `Enter` never sends the draft from a different repo.
- After you send the draft or put it away, the popup opens where you are again.
- Words that you put away with `Esc` also keep their repo. `Ctrl+Z` in a popup that opened in a different repo takes the
  board to the repo that the words were typed for, and says so.
- If that folder is gone, the board stays where it is. It says which repo it now sends the words from.

### A board in a tab and a popup at the same time

- A board in a tab continues to run next to the popup.
- Each one has its own note of where it is, and its own selection, filter and words.
- The popup keeps its view in `view-popup.json`. The first popup starts from the view of the tab, without its draft.
- There is one keeper, one Watcher and one Coordinator, whichever board you look at.
- The board that was shown for the focused pane answers the approve action for that pane.

### When Herdr does not open the popup

If Herdr does not open the popup, the board opens in a tab and says why on its message line. Herdr has one popup for
all plugins. While the popup of another plugin, or a `type = "popup"` command of your own config, is open, the popup of
the board cannot open.

### The old meaning of popup

Before, `popup` was a pane in the overlay of Herdr. That pane filled the tab that it opened over, and stayed with that
tab. That is gone, and `popup` now means the popup on this page. There is no third value. A setting that says `popup`
needs no change.

### How the popup differs from the own menu of Herdr

All of this applies to Herdr 0.9.3, and to its newest build, 2026-10-12. In these versions, a plugin has no other way to
draw over the window
([Herdr 0.9.3 facts: the popup](../contributing/checked-on-herdr-popup.md#the-popup-for-the-board)).

- **The own keys of Herdr do not work while the popup shows.** Herdr gives its popup every key, the prefix and the
  board key too, and starts no action.
- Thus the board reads its own key itself. It gets the key from the `config.toml` of Herdr (the prefix key, then the key),
  and closes when you press it.
- No other key of Herdr does anything in the popup. Close the popup first.
- A prefix key that you press alone does nothing, as in Herdr.
- **The same applies in another popup.** While the popup of another plugin shows, the board key is typed into that
  popup. Herdr starts no action, so no code of the board runs. The board cannot open, and it cannot say why. Close that
  popup first.
- **The popup covers the panes, not the sidebar and the tab bar of Herdr.** Herdr puts its popup in the centre of the
  part of the window that has the panes. The sidebar and the tab bar take no click while the popup shows.
- **Herdr ties the popup to the tab that it opened over.** Herdr hides it, still running, when another tab shows. The
  board closes it at that moment, so you never see that. For this reason, the popup closes, and does not follow, when
  a command moves Herdr to a different place.
- **Nothing behind the popup is dimmed.** The board chooses the size of the popup, not the menu of Herdr.

What Herdr needs for the real thing: [missing in Herdr](../contributing/herdr-0.9.3-facts.md#missing-in-herdr).

## Related pages

- [The header and the list](board-header-and-list.md)
- [The chat of the board](board-chat.md)
- [Settings](settings.md)
- [Tabs](tabs.md)
- [The Coordinator](../explanation/coordinator.md)
- [Herdr 0.9.3 facts](../contributing/herdr-0.9.3-facts.md)
- [Why the board looks as it does](../explanation/board-design.md)
