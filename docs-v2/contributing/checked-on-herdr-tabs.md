# Herdr 0.9.3 facts: terminal sizes, tabs and the Coordinator

This page holds the facts about terminal sizes, tabs and spaces in Herdr 0.9.3, and about how the Coordinator and the
keeper run there. Each entry is one fact, with what a real test showed. For the list of all facts, see
[what was checked on Herdr 0.9.3](herdr-0.9.3-facts.md).

These facts are from the second part of the notebook, checked on 2026-10-09 (see
[the turn end](checked-on-herdr-turn-end.md)). Where an entry gives its own date, that date applies to it.

## Terminal sizes

### Who sets the size of the terminal of a pane

- The own window of Herdr sets it, until a `herdr terminal attach` is on that terminal.
- Then the terminal has the size of the attach (93×33 became 200×50). It keeps that size when the window of Herdr is
  resized. It gets the size of the window again the moment that the attach ends.
- Herdr names no attached viewers anywhere.
- `pane layout --pane <id>` gives the place of the pane in the window of Herdr. Its terminal is that place less up to
  3 columns and 2 rows (frame, scrollbar). For the zoomed pane (the `focused_pane_id` of the layout when `zoomed`), it
  is the whole `area` of the tab less the same.
- A second full Herdr client changes the layout and the terminal together. Thus the board does not mistake it for an
  attach.

### Whose size the terminal of a pane has, with two Herdr windows on one session

Seen on 2026-10-12, with two clients of 170×48 and 54×26 in a scratch session, and in the log and on the terminal of a
real session.

- The terminal has the size of the window that was last typed in or resized. The terminal of every pane takes the
  size of that window at once. It goes back when the other window is typed in.
- `pane layout` answers for the window that last did either.
- If the other window is only resized, the two disagree. The layout is that of the resized window, and the terminal is
  still that of the first window. This continues until a key is typed.
- `terminal session control --takeover` changes nothing then. The size is its own only while the control is held.
  Herdr puts the terminal back as it leaves.
- The log of a real session shows the windows as `client connected … cols=47 rows=44` and `cols=138 rows=44`. The
  terminal of the board went 141×46 → 138×44 → 141×46 within 0.6 s.

### Helpers that start and end with two windows of different sizes on the session, one of them remote

Seen on 2026-10-12 (`herdr --remote`) in a scratch session, with the resize events of the pane logged.

- No resize occurred. `workspace create`, `tab create`, `agent start`, `tab close` and `workspace close` in the
  background changed the size of no pane.
- This was true with the local window typed in last, with the remote window typed in last, and after the remote
  window was only resized.
- A pane spawned at that time gets the size of the window that has the session.

### What a helper of the board does to the pane of the board

Seen on 2026-10-12, with a real board watched 20 times a second.

- Nothing. These leave the terminal size and the `pane layout` of a pane in the focused space as they were:
  - `workspace create`, `worktree create` and `pane split` with `--no-focus` in another space;
  - `agent start` there;
  - `pane close` and `workspace close`.
- This was true through the whole life of a task with a reviewer, and through a Coordinator started and stopped.
- A pane split off the own pane of the board halves it.

## Tabs and spaces

### `herdr tab create --workspace <id> --cwd <folder> --label "Reviewer #1" --env KEY=VALUE --no-focus`

Seen on 2026-10-12, in a separate scratch Herdr.

- It answers the new tab and its one pane (`tab`, `root_pane`). The pane is at its shell, in that folder, with the
  variable set.
- The focus stays where it was (`tab list`: the first tab is still `focused`).
- The pane of the first tab keeps its rectangle in `pane layout` (120×40 before and after). A split halves it.
- `tab rename <id> Agent` renames a tab. The first tab of a new space is labelled `1`.
- When the last pane of a tab closes, the tab closes. `tab close <id>` closes the tab with its panes.
- Herdr does not give the id of a closed tab again (`w1:t2` closed, and the next was `w1:t3`). `tab close` on a tab
  that is gone answers `tab_not_found`.

### Several worktrees of one repo as tabs of one space

Seen on 2026-10-13, in a separate scratch Herdr, with Claude Code, Codex and pi each in its own tab.

- A space made by `worktree create` takes more tabs whose `--cwd` is another worktree of the same repo
  (`tab create --workspace <id> --cwd <that worktree>`). An agent started in each tab works in its own worktree.
- If you close the first tab of the space (the tab that `worktree create` made), the space and its other tabs stay as
  they are. This is also true when the tab that stays is in another worktree.
- `workspace close` then closes the space as any space.
- This is how a task done several times at once has `Agent A`, `Agent B` and `Agent C` in one space (see
  [attempts](../explanation/attempts.md)).

### The home space, and a pane moved into a tab

Seen on 2026-10-12, in a separate scratch Herdr, with Claude Code running in the pane.

- `workspace create --cwd ~` with no label makes a space labelled `~`.
- The label follows the folder of the pane of the first tab of the space. It was `tmp` after `cd /tmp` there. It was
  the name of the folder of the Coordinator when its tab was the only one left. Thus it is not a fixed name.
- `--no-focus` leaves the focus where it was, but not for the very first space of a session.
- `pane move <pane> --new-tab --workspace <id> --label Coordinator --no-focus` puts the pane in a new tab of that
  space, and its program continues to run:
  - The terminal id is the same.
  - The agent keeps its name and status.
  - The focus and the rectangles of the other panes stay as they were.
  - The space that the pane left closes if that was its last pane (`closed_workspace_id`).
- The pane has a new id (`w4:p1` became `w1:p5`). `pane get` still answers for the old id. `agent get` does not. The
  program itself still has the old id in `HERDR_PANE_ID`.
- `tab list --workspace <id>` gives the label and the pane count of each tab.
- The label of a tab is plain text. `tab rename <id> ""` empties it. A tab that was never named is labelled with a
  number. Thus you can rename a tab back to restore its label.
- Live with the code of the board:
  - A first message made the home space and the `Coordinator` tab, with the focus left on another space.
  - A Coordinator was put back into a space of its own, and the home space was closed. The next message moved it into
    a tab of a new home space. It answered from the same conversation.
  - The tab of the board was `herdr-projects` while the board was open, and `notes` again after `Ctrl+C` two times.
    `herdr-projects list` renamed nothing.
- `pane close` on the last pane of a space closes the space (`workspace list` no longer has it).
- `pane list` with no `--workspace` lists the panes of every space, with their folder.

### `herdr workspace create --env KEY=VALUE`

- The agent started in that space has the variable. Thus the commands of the Coordinator reach a board kept in another
  data folder.

## The Coordinator and the keeper

### A long launch command through `herdr agent start`

- Herdr types the command into the shell of the pane, and a terminal takes at most 4096 bytes on one line.
- The rules of the Coordinator inside the `-c developer_instructions=…` of Codex arrived cut off, at a shell
  continuation prompt. The start timed out.
- Now the flag only points at a rules file. At first, the rules also went in with the first message. Since the box
  became a word-for-word passthrough, they do not. This was not yet run against a real Codex.
- Three new starts and two restarts of Codex in a row came up in about four seconds each. The restarts were in the
  same conversation.

### The own new-chat command of the agent (Claude Code `/clear`, Codex `/new`, pi `/new`)

- Each starts another conversation file for the same folder, and leaves the first file as it was. For pi, the file is
  `<time>_<id>.jsonl`, whose first line names the working folder and the id. `--session-id` opens it again.
- The Coordinator has a folder in which nothing else chats. Thus the newest conversation saved there since the
  Coordinator started is the one that it is in.
- Live, with all three programs: "pineapple", new chat, "watermelon". The board showed watermelon. After a restart,
  the agent still answered watermelon.

### Other conversations saved for the folder of the Coordinator

- A helper agent that Codex spawns saves its own conversation there, newer than the main one. Its first line has
  `"source": {"subagent": …}` and `parent_thread_id` (seen in real files of 0.160.0).
- A headless `codex exec` has `"source": "exec"`.
- Claude Code keeps the side conversations of helpers in a `subagents` subfolder. Each entry names its `entrypoint`
  (`cli` for the interactive program).
- The board takes only a chat of the interactive program itself as the chat of the Coordinator.

### The Coordinator on Claude Code and on Codex, live

- Claude Code: the first message started it. More messages went into the same conversation, from the text box and
  from its own pane. A task was added in the named repo. After a restart, it still knew an earlier answer
  (`--resume`).
- Codex, one time only: one message, and the task was added. The board found its own conversation from its folder and
  its start time, and read its last message.
- pi: started as the Coordinator, with three messages, its own /new and a restart (see the new-chat command above).

### A keeper started from the startup hook

- It survives the hook, and it replaces itself on an update.
- Nobody waited "for hours" to test it. Thus hook events and the board also start it.

## Related pages

- [What was checked on Herdr 0.9.3](herdr-0.9.3-facts.md)
- [The Coordinator](../explanation/coordinator.md)
- [Attempts](../explanation/attempts.md)
- [Tabs](../reference/tabs.md)
- [Board size](../explanation/board-size.md)
