# What was checked on Herdr 0.9.3

This page is the lab notebook behind the board. You do not need it to use the board. Each entry is one thing that the
board relies on, about Herdr or about an agent program in Herdr. The entry also tells what a real test on Herdr 0.9.3
showed.

Other pages link here for the proof. To find a fact, look for its words in the tables below, then open its page. The
last section, [missing in Herdr](#missing-in-herdr), lists what Herdr needs before the popup of the board can work as
the own menu of Herdr does.

## The facts

### Plugin panes, the popup and the keyboard

| Fact | Page |
| --- | --- |
| Popup panes; `HERDR_PLUGIN_CONTEXT_JSON` in a popup | [The popup](checked-on-herdr-popup.md) |
| `agent focus` from inside a popup holds after it closes | [The popup](checked-on-herdr-popup.md) |
| A popup pane while another tab shows | [The popup](checked-on-herdr-popup.md) |
| How the own menu of Herdr is drawn, and what a plugin gets for it | [The popup](checked-on-herdr-popup.md) |
| The popup of Herdr, tried for the board | [The popup](checked-on-herdr-popup.md) |
| The board key with `board.open` set to `popup`, live | [The popup](checked-on-herdr-popup.md) |
| A tab or space changed while the popup of the board was still starting | [The popup](checked-on-herdr-popup.md) |
| A plugin pane opened with `--placement tab` | [Plugin panes and the keyboard](checked-on-herdr-panes.md) |
| A plugin pane opened with `--placement overlay` | [Plugin panes and the keyboard](checked-on-herdr-panes.md) |
| The board key with `board.open` set to `popup` while that meant the overlay | [Plugin panes and the keyboard](checked-on-herdr-panes.md) |
| The board key with `board.open` left at `tab`, live | [Plugin panes and the keyboard](checked-on-herdr-panes.md) |
| Giving one pane the keyboard by its id | [Plugin panes and the keyboard](checked-on-herdr-panes.md) |
| A second viewer on the terminal of a pane (`herdr terminal attach`) | [Plugin panes and the keyboard](checked-on-herdr-panes.md) |

### Agents and their start

| Fact | Page |
| --- | --- |
| Plugin event hooks on `pane.agent_status_changed` | [Agents](checked-on-herdr-agents.md) |
| `agent start`; `state_change_seq`, `completion_seq` in `agent get` | [Agents](checked-on-herdr-agents.md) |
| When `agent start` says "ready" | [Agents](checked-on-herdr-agents.md) |
| What `agent prompt` types | [Agents](checked-on-herdr-agents.md) |
| A prompt typed into Claude before it has drawn its screen | [Agents](checked-on-herdr-agents.md) |
| A prompt typed into a running Claude at 3% of a processor, or with paste and enter in one write | [Agents](checked-on-herdr-agents.md) |
| A message typed into Claude in the middle of a long turn | [Agents](checked-on-herdr-agents.md) |
| `agent_session` in `agent get` | [Agents](checked-on-herdr-agents.md) |
| Workspace and pane ids across a server restart | [Agents](checked-on-herdr-agents.md) |
| Which event fires when an agent exits to its shell | [Agents](checked-on-herdr-agents.md) |
| A process spawned detached from a hook | [Agents](checked-on-herdr-agents.md) |
| `PATH` in processes that a hook spawns | [Agents](checked-on-herdr-agents.md) |
| `worktree create --cwd <repo>` with no workspace open for it | [Agents](checked-on-herdr-agents.md) |
| Claude and Codex in a brand-new worktree with full access | [Agents](checked-on-herdr-agents.md) |
| Codex after a reply | [Agents](checked-on-herdr-agents.md) |
| `herdr machine list --json` with a saved machine | [Agents](checked-on-herdr-agents.md) |
| Codex before 0.156.0 under `herdr agent start` | [Agents](checked-on-herdr-agents.md) |
| The other programs of Herdr under `herdr agent start --kind <name>` | [Agents](checked-on-herdr-agents.md) |
| kilo 7.8.12 and qodercli 1.1.72 installed with npm; Letta Code | [Agents](checked-on-herdr-agents.md) |
| omp 18.11.1 without the integration of Herdr for it | [Agents](checked-on-herdr-agents.md) |
| A task on droid 0.242.0 through the board | [Agents](checked-on-herdr-agents.md) |
| A long start command or message | [Agents](checked-on-herdr-agents.md) |

### The turn end and the conversation files

| Fact | Page |
| --- | --- |
| Claude `--settings` with Stop and StopFailure hooks, through `herdr agent start` | [The turn end](checked-on-herdr-turn-end.md) |
| `claude --settings` adds to the hooks of the user | [The turn end](checked-on-herdr-turn-end.md) |
| A hook that fails, is noisy or is slow | [The turn end](checked-on-herdr-turn-end.md) |
| Claude resumes a known session id (`--resume <id>`) | [The turn end](checked-on-herdr-turn-end.md) |
| The conversation file of Claude | [The turn end](checked-on-herdr-turn-end.md) |
| Codex obeys `-c notify=[…]` at launch | [The turn end](checked-on-herdr-turn-end.md) |
| The conversation file of Codex | [The turn end](checked-on-herdr-turn-end.md) |
| A turn that the provider cut off, in the three files | [The turn end](checked-on-herdr-turn-end.md) |
| The fast mode of Codex | [The turn end](checked-on-herdr-turn-end.md) |
| pi `-e <file>` | [The turn end](checked-on-herdr-turn-end.md) |
| A launch flag for standing rules | [The turn end](checked-on-herdr-turn-end.md) |
| A rules file in the start folder (the rules of the Coordinator) | [The turn end](checked-on-herdr-turn-end.md) |
| A conversation after its folder was moved | [The turn end](checked-on-herdr-turn-end.md) |
| User-level skills folders | [The turn end](checked-on-herdr-turn-end.md) |

### The sidebar, the config and the screen

| Fact | Page |
| --- | --- |
| The sidebar of Herdr shows the pane and workspace metadata of the plugin | [The sidebar](checked-on-herdr-sidebar.md) |
| How the sidebar of Herdr draws a row of several values | [The sidebar](checked-on-herdr-sidebar.md) |
| How long a metadata token lives | [The sidebar](checked-on-herdr-sidebar.md) |
| Telling Herdr that a task is blocked with `pane report-agent --state blocked` | [The sidebar](checked-on-herdr-sidebar.md) |
| A new `[[keys.command]]` after `herdr server reload-config` | [The sidebar](checked-on-herdr-sidebar.md) |
| The config path and the default keys of Herdr | [The sidebar](checked-on-herdr-sidebar.md) |
| What the terminal shows, inside Herdr | [The sidebar](checked-on-herdr-sidebar.md) |
| The mouse in a pane | [The sidebar](checked-on-herdr-sidebar.md) |
| The theme of Herdr, for a plugin | [The sidebar](checked-on-herdr-sidebar.md) |

### Terminal sizes, tabs and the Coordinator

| Fact | Page |
| --- | --- |
| Who sets the size of the terminal of a pane | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |
| Whose size the terminal of a pane has, with two Herdr windows on one session | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |
| Helpers that start and end with two windows of different sizes, one of them remote | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |
| What a helper of the board does to the pane of the board | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |
| `herdr tab create --workspace <id> --cwd <folder> --label … --env … --no-focus` | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |
| Several worktrees of one repo as tabs of one space | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |
| The home space, and a pane moved into a tab | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |
| `herdr workspace create --env KEY=VALUE` | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |
| A long launch command through `herdr agent start` | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |
| The own new-chat command of the agent (Claude Code `/clear`, Codex `/new`, pi `/new`) | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |
| Other conversations saved for the folder of the Coordinator | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |
| The Coordinator on Claude Code and on Codex, live | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |
| A keeper started from the startup hook | [Terminal sizes and tabs](checked-on-herdr-tabs.md) |

## Missing in Herdr

Herdr 0.9.3, and its main branch of 2026-10-13, lack the items that follow. Without them, the popup of the board cannot
be what the own menu of Herdr is. Each item is something to raise with Herdr. A plugin cannot replace any of them.

- **A popup that belongs to the window, not to a tab.** The server keeps the tab that a popup opened over, and hides
  the popup when another tab shows. Herdr could keep a popup with the client that opened it, over whatever that
  client shows. Then the plugin would not need to close the popup when the tab changes.
- **The keys of Herdr while a popup has the keyboard**, or at least the key that opened it. Now every key goes to the
  program of the popup. Thus a plugin must read the config of Herdr to know its own key, and no other key of Herdr
  works. The same gap is why the board key does nothing while the popup of another plugin shows. An event to plugins
  that a key was held back would at least let the board say so. (Task 135 noted it as "keys swallowed by floating
  windows". This work meets it again, and works around it only for the own key of the board.)
- **A way to close the popup from the command line** (`herdr popup close`). The socket has `popup.close`, but the
  command line does not.
- **Whose popup is open.** `ui_busy` does not tell, and `popup.close` closes the popup of anybody. Thus the board closes
  only a popup that its own running board stands behind.
- **A popup over the whole window, with the sidebar and the tab bar, and with what is behind it dimmed.** This is
  for the case that the menus of Herdr get that one day. Now the popup of a plugin is in the middle of the area of the
  panes, and nothing is dimmed.

### Settled by this work

A zoomed pane lost its zoom when the popup of the board closed. Task 135 noted this as the other problem. The overlay
caused it. The popup of Herdr does not change the layout.

## Related pages

- [The popup](checked-on-herdr-popup.md)
- [Plugin panes and the keyboard](checked-on-herdr-panes.md)
- [Agents and their start](checked-on-herdr-agents.md)
- [The turn end and the conversation files](checked-on-herdr-turn-end.md)
- [The sidebar, the config and the screen](checked-on-herdr-sidebar.md)
- [Terminal sizes, tabs and the Coordinator](checked-on-herdr-tabs.md)
- [Crates](crates.md)
- [How the tests work](testing.md)
