# How the board writes Herdr's sidebar

This page tells how the board puts its entries in Herdr's sidebar, and why some things of the board cannot show there.
For what each entry shows, see [Herdr's sidebar](../reference/sidebar.md).

## Why the sidebar copies the row

Each entry in the sidebar is a small copy of the row on the board, in the same words. The entry does not show the
second line of the row, because row 2 of the entry already shows the state.

## Why the dot is Herdr's

The board writes no state to Herdr. If the board told Herdr that a task is blocked, Herdr would stop watching the
pane ([Herdr 0.9.3 facts](../contributing/herdr-0.9.3-facts.md)). Thus the dot is Herdr's own reading of the pane, and
the words next to it give the exact state.

## Limits with Herdr 0.9.3

- **The first row of the Coordinator is `~ · Coordinator`.** The Coordinator is in a tab of the home space. A sidebar
  row can hide a value only by the value itself. If the rows hid `~`, they would hide it for every agent of yours in
  the home space.
- **No marks of the board's own.** The icon of an entry is Herdr's state icon. Only Herdr's own five states have one.
  Thus the words give the state.
- **No colour by status.** Herdr can colour a value by rules on its text, which you set in your config. The rows of the
  plugin do not use this, except to dim the message of the Coordinator.
- **Not in the collapsed sidebar or in a window of phone width.** Those keep Herdr's own compact layouts, and sidebar
  rows do not change them. Herdr's configuration page says so. This was not tested here.
- **The tasks of another machine.** The plugin on that machine writes them into the Herdr of that machine. Thus they
  show here only when this Herdr connects to that machine, and that machine runs this version of the plugin.

## How the board writes the entries

- **One writer, no words of its own.** The board has one writer for all entries
  ([`crates/flow/src/flow/sidebar.rs`](../../crates/flow/src/flow/sidebar.rs)). It draws the row of the task with the
  board's own code, without colours
  ([`crates/flow/src/flow/sidebar_entry.rs`](../../crates/flow/src/flow/sidebar_entry.rs)). The words of row 2 come
  from the one list of status words ([`crates/core/src/status_words.rs`](../../crates/core/src/status_words.rs)).
- **When.** The process that ticks (the keeper or an open board) writes an entry once a second if a word of it
  changed. That is as often as the board draws its own rows again. When a task starts to wait for you, goes wrong,
  starts again or ends, the process that did the change writes the entry at once. It does not wait for the tick.
- **No conflicts.** The Watcher and the check only keep what they find on the task ([the Watcher](watcher.md),
  [check progress](check-progress.md)). They write nothing to Herdr. Thus no two writers write the same line.
- **After a Herdr restart.** Herdr keeps none of the entries when its server restarts. The board sees this on its
  next tick, by Herdr's own reading of the pane: the shown name that the board wrote is gone. Then the board writes
  every entry of that Herdr again.
- **Nothing old stays.** The board replaces what it wrote. It never lets an entry expire.
  - A task that is stopped, done or moved on gets its new words at once, or in the same second.
  - An entry goes with its pane: Herdr removes what was written about a closed pane.
  - If a task is archived or removed while its pane is still open, the board removes its words from the pane.
- **As what.** The board writes the entries as Herdr's display metadata, from the source `plugin:herdr-projects`. It
  uses `herdr pane report-metadata`, `herdr workspace report-metadata` and the name of the space.
  - Row 2 is the shown name of the agent.
  - Row 3 is the token `progress`.
  - The token `summary` is only for the Coordinator. On the pane and the space of a task, the board clears it. Thus a
    line that an older version wrote goes away automatically.

## Related pages

- [Herdr's sidebar](../reference/sidebar.md)
- [The Watcher](watcher.md)
- [Check progress](check-progress.md)
- [Herdr 0.9.3 facts](../contributing/herdr-0.9.3-facts.md)
