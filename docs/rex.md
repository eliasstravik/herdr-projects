# Rex

The board can run a task's space, tabs and agents in [Rex](https://rex.superlogical.com) instead of herdr. herdr
stays the default. Pick Rex per repo, or per task, with one setting:

```sh
herdr-projects set workspace.terminal rex --repo .
```

The choice is read when the task's space is made and kept on the task (`link.socket` is `rex:`, or
`rex:<host>` for another Rex server), so tasks under way keep the terminal they started in.

## How the board's parts map onto Rex

| The board's part | In herdr | In Rex |
|---|---|---|
| A project (a repo) | a set of spaces | one Rex session, `hp-<repo>` |
| A task's space | a workspace (herdr makes the worktree) | a window opened in the worktree (`-c`); the board makes the worktree with `git worktree add`, under `<data folder>/worktrees/<repo>/<branch>` |
| A tab | a tab | a window of that session |
| A pane | a pane | a block, named after its agent (`block rename`) |
| The board | a plugin tab or popup | a block in a window called `board` |
| An agent's state | herdr's own integration | what the program reports itself, OSC 7501 (`program_status`) |

What Rex does not hold (which session is a project's, which windows make a space, which block runs which agent and
its turn counters) the board keeps in `<data folder>/rex/<server>.json`.

Every block the board makes starts its shell with the board's environment and `HERDR_PROJECTS_SOCKET=rex:…`, so an
agent's `herdr-projects report` and the board opened in the `board` window talk to the same Rex server.

## What is different

- **Turns** are seen only through the program's own OSC 7501 reports (`state=working`, `idle`, `done`, `blocked`).
  A program that does not report its state shows as `unknown`, and its turns are not seen. Rex sends no events to
  the board: the board's keeper reads the state every few seconds.
- **Prompts** are pasted (bracketed paste) and entered with a key press of their own.
- **Not on Rex**: moving a pane to another space, handing a block's terminal over, herdr's sidebar summaries, the
  board as a popup (it opens in its `board` window), other machines (use `rex:<host>`), tab events.
- The `rex` program is `REX_BIN`, else `rex` on the `PATH`, else the one inside `/Applications/Rex Beta.app`.

The code: `crates/core/src/herdr/backend.rs` (the one interface, `TerminalBackend`), `herdr_cli.rs` (herdr) and
`crates/core/src/herdr/rex/` (Rex).
