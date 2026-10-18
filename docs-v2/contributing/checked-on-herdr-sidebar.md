# Herdr 0.9.3 facts: the sidebar, the config and the screen

This page holds the facts about the sidebar, the metadata, the config, the keys, the colours and the mouse of Herdr
0.9.3 that the board uses. Each entry is one fact, with what a real test showed. For the list of all facts, see
[what was checked on Herdr 0.9.3](herdr-0.9.3-facts.md).

These facts are from the second part of the notebook, checked on 2026-10-09 (see
[the turn end](checked-on-herdr-turn-end.md)). Where an entry gives its own date, that date applies to it.

## The sidebar and metadata

### The sidebar of Herdr shows the pane and workspace metadata of the plugin

- **Not by default.** State labels and the `$summary` and `$progress` tokens show only when `[ui.sidebar.*] rows` in
  `config.toml` lists them.
- Only `--display-agent` and the name of a space show with no setup.
- Thus the shown name of the agent tells where a task is. The name of its space holds the number and the title of the
  task. The first run offers the rows for the rest (see [the sidebar](../explanation/sidebar.md)).

### How the sidebar of Herdr draws a row of several values

Tried on 2026-10-13, in a scratch session inside tmux with a real Claude Code in the pane. After that, the whole walk
of a task in [the sidebar](../explanation/sidebar.md) was tried.

- Herdr joins the values with ` · `.
- If a row is wider than the sidebar, Herdr shares the width between its values. It cuts each value at its own end
  with `…`. A short value stays whole (`Reading the side… · 8%`). Two long values are both cut
  (`Reading t… · 8% 16m l…` in the default 26 cells of Herdr).
- Thus several facts that must be cut only at the end go into one value. `agent · working · R#1 PR#113` is one value,
  the shown name of the agent. The number of a task is first in the name of its space.
- A rule with `hide = true` removes a value only by its own text.
  `{ token = "tab", rules = [{ equals = "Agent", hide = true }, { starts_with = "Reviewer #", hide = true }] }` hid the
  tab names of a task and no others. Nothing hides one value because of another value.
- The sidebar stayed 26 cells wide with a space named in 60 letters. No widening by name was seen.
  `sidebar_width = 40` with `sidebar_max_width = 40` made it 40 cells wide.
- `workspace rename <id> <text>` renames a space, and its row follows at once.
- `--display-agent`, a token, and the name of a space written from a command showed within one second, with no
  reload.
- An empty value does not show, and its ` · ` goes with it. A row with no value is not drawn.
- The dot beside an entry never changed with anything written in this way.
- `pane list`, `pane get` and `agent get` give back the `display_agent` and `tokens` of a pane. After
  `herdr server stop` and a new start, the panes are there with neither (`display_agent: null`). One tick of the board
  wrote them again.

### How long a metadata token lives

Tried on 2026-10-13 in a scratch session, with `pane report-metadata` and `workspace report-metadata` with `--ttl-ms`.

- A token written with a time to live is gone when that time ends, by itself (`pane get` no longer has it). This is
  true whatever else is written in the meantime.
- These do not cancel its expiry: a later report of state labels, a report of another token, `--clear-token` of other
  names, and a report from another source.
- If the same token is written again without `--ttl-ms`, it stays until it is cleared. If it is written again with
  `--ttl-ms`, its time starts again.
- State labels written with no time to live stay longer than a token.
- Thus a token never lives longer than its time. A line that stays longer was written again, or written to stay.
- The error was on the side of the board. Two things were wrong:
  - Nothing removed the working line of the Watcher (`summary`, `progress`, `eta`, three minutes each) from a pane at
    the moment its task stopped working.
  - The `summary` of the space had two writers: the label (written to stay) and the line of the Watcher (written to
    run out).
- Both are settled in `crates/flow/src/flow/sidebar.rs`, which is now the only writer. It writes everything to stay,
  and replaces it when a word changes.

### Telling Herdr that a task is blocked with `pane report-agent --state blocked`

Tried on 2026-10-13, in the same session, with a real Claude Code in the pane, source `plugin:herdr-projects`.

- The agent of the pane reads `blocked` at once (`state_change_seq` one up). Herdr then stops reading the screen of
  the agent.
- Words typed into the pane (`pane send-text`, then Enter) made the agent work and answer. Herdr said `blocked` all
  the time, with no status event.
- In the meantime, Herdr refuses `agent prompt` (`agent_blocked`: "is blocked and requires interactive input").
- `pane release-agent` gives the state back in three steps, also when nothing occurred between them:
  1. `idle` with a new `completion_seq` (a finished turn that never occurred);
  2. `unknown`;
  3. `idle` with no `completion_seq` at all.
- A report of `idle` over the `blocked` counts a completion in the same way.
- Thus the report takes over the whole status of the pane, as the docs of Herdr say of `report-agent`: "when an
  integration also reports state, herdr uses those reports instead of reading the screen". The docs tell hooks beside
  an integration to use metadata in its place.
- The board does **not** use it, for three reasons:
  - Its hook must see that an agent starts to work again when you type in the own pane of the agent.
  - Herdr refuses `agent prompt` for a blocked agent.
  - A `completion_seq` that jumps on release looks to the hook like a turn that it missed.
- Thus the own dot of Herdr for a task that waits stays the dot of Herdr (done or idle). The label and the line beside
  it tell where the task is.

## The config and the keys

### A new `[[keys.command]]` after `herdr server reload-config`

- It works without a restart.
- Herdr reports a broken file as `failed`, and keeps the old config. A key bound two times is `partial`.
- `herdr config check` validates a candidate that `HERDR_CONFIG_PATH` gives.

### The config path and the default keys of Herdr

- The config path is `~/.config/herdr/config.toml`.
- `herdr --default-config` lists most built-in keys: `prefix+w` is the workspace picker of Herdr, and `prefix+a` is
  free.
- It does not list `copy_mode` (`prefix+[`) or `swap_pane_left/down/up/right` (`prefix+shift+h/j/k/l`), which the
  config reference has. `crates/core/src/herdr/default-keys.toml` holds them all.
- A binding can be one key or a list. Herdr still reads `[keys.indexed]`.
- `herdr config check` does not report a key bound two times.

## The screen

### What the terminal shows, inside Herdr

- The board asks the terminal for its background (OSC 11) and its 16 colours (OSC 4). It does this when the
  `theme.auto_switch` of Herdr is on, or when the theme paints with the own colours of the terminal.
- Herdr answers for its panes with the colours of the terminal that the client runs in.
- With no answer, the board uses dark and the colours of xterm. (The scratch session has no real terminal behind it.)
- `theme light` chooses by hand.

### The mouse in a pane

Tried on 2026-10-12 in a scratch session inside tmux, with mouse codes typed into the client.

- A full-screen program that has not asked for the mouse: Herdr sends it the wheel as `↑` and `↓`, one for each step,
  with no position. A drag selects in the pane and copies (the own selection of Herdr).
- A program that asked for the mouse (`?1000h`, with or without `?1002h`): Herdr gives it everything, in the
  coordinates of the pane. This is the wheel, press, drag and release, also with `Shift` held. Herdr selects nothing
  itself.
- A clipboard code (OSC 52) written in a pane goes on to the terminal, with the note `copied to clipboard` of Herdr.
- Thus the board asks for the mouse, takes the wheel, and selects and copies itself (see
  [the mouse](../explanation/mouse.md)). These were all seen on the real board in a pane: the wheel over the chat and
  over the list, `Alt+U` `Alt+J`, and a drag that copied two lines.
- Not tried: the popup (a plugin pane with `--placement overlay` needs the plugin installed in Herdr), and the `Cmd`
  and the click on a link of a real Mac terminal.

### The theme of Herdr, for a plugin

- Herdr does not give its theme to plugins.
- The board reads the `[theme]` and `[ui]` tables that Herdr reads, with a full TOML reader (the `toml` crate). It
  resolves them as Herdr 0.9.3 does (`src/app/state.rs` palettes, `src/client/shell.rs` marks).
- The marks are `●` for working, blocked and done, and `○` for idle. With `status_indicators = "symbols"`, the marks
  are `◐ × ✓ ○`.
- Text is the `text` colour of Herdr. The side panel is on `sidebar_bg` when the theme sets one.
- The review page uses the theme in force when the page is written. Where Herdr follows the terminal (`auto_switch`,
  the `terminal` theme), the page follows the light or dark setting of the browser.
- A theme changed in Herdr shows the next time that the board opens.
- Marks, group headings, the bar and the percent use the colour of Herdr for the state of the task: working `yellow`,
  blocked `red`, done `teal`, idle `green`, and under review also `green`.
- The board's own blue, purple and pink are not colours of Herdr. They are for a review that runs, an automation, and
  a task that waits for a free slot (see [colours](../explanation/colours.md)).
- The title of a task is always the text colour of Herdr, whatever its state.
- What the row of a task says after the title, and each label in the task panel, is greyed out (`overlay0`). In the
  `terminal` theme it is also faint, because the grey of that theme is the text colour of most terminals.
- The headlines of the panel are bold text. Its rules and review boxes are `surface1`. A setting that is only your
  default is `subtext0`.
- Text, secondary text, rules and the selected row use the sidebar colours of Herdr (`text`, `subtext0`, `overlay0`,
  `surface_dim`, `selection_bg`/`active_row_bg`).
- The score of a review is `green` when the reviewer says merge, and `red` when it says do not merge. The colour is
  the advice: there are no words for it.
- With no theme set, Herdr shows Catppuccin (`herdr --default-config`: `# name = "catppuccin"`), and so does the board.
- A Herdr opened with `--remote` paints with the config of the other machine, which the board cannot read (see
  [colours](../explanation/colours.md)).
- A task that nobody started is `todo`, a dim grey of the board's own.
- The frame of the text box is `text` for the Coordinator, and the status colour of the task for its agent.

## Related pages

- [What was checked on Herdr 0.9.3](herdr-0.9.3-facts.md)
- [The sidebar](../explanation/sidebar.md)
- [Colours](../explanation/colours.md)
- [The mouse](../explanation/mouse.md)
- [Add your own keys](../how-to/add-your-own-keys.md)
- [The Watcher](../explanation/watcher.md)
