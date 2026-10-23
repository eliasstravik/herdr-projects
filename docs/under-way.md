# What a key shows at once

**A key that acts on a task shows it right away**, even when the work takes a while (setting up a worktree, starting
an agent, a merge, another machine answering). The row gets the working mark and says what is happening, in the
working colour. The task panel says the same under Status, as `Now`.

| Key | The row says |
|---|---|
| `Ctrl+S`, or a first reply to a task with no agent | `Starting…` |
| `Ctrl+X` on a running task | `Stopping…` |
| `Ctrl+X` then `y` on one that is not running | `Removing…` |
| `Ctrl+L` | `Merging…` (after the `y`), `Approving…`, `Trying again…` or `Finishing as it is…`, depending on what it says yes to |
| `Ctrl+R` | `Sending back…` |
| `Ctrl+A`, `Ctrl+Z` | `Archiving…`, `Restoring…` |
| `Ctrl+P` (on a task, and on a machine's or a project's row, here or on its machine) | `Pinning…`, `Unpinning…` |
| `y` when picking an attempt | `Keeping attempt B…` |
| `Alt+O` | `Opening…` (until its pane opens) |
| `Space`, `Ctrl+S`, `Ctrl+X` on an automation | `Turning on…`, `Turning off…`, `Firing…`, `Removing…` |
| `Ctrl+S`, `Ctrl+X`, `Ctrl+O` on a machine's or a project's row (its coordinator, here or on its machine) | `Starting…` (started, or autopilot on), `Stopping…` (stopped, or autopilot off), in the time's place with the working mark |
| `Alt+O` on a machine's or a project's row | `Opening…` |

**Nothing is said under the chat when it worked**: no confirmation line, for any key. The row shows what is under way,
then what it became. What went wrong is still said (below).

## When it ends

The words stay until the board has read the task again after the work ended. Then the row shows the task's real state.
A read that started before the work ended does not count, so the row never flickers back to the old state.

For another machine's task, the words stay until that machine shows the task changed, or has answered twice (at most
15 seconds).

## When it fails

The row says so in red, with the error mark: `Could not stop: …` and why. The panel says the same under `Now`, and the
message line says it until you press `Esc`. The row goes back to normal after ten seconds, or when `Esc` dismisses the
error.

## Pressing the key again

It does nothing while the work is under way, so nothing starts twice. The message line says
`Still starting PRJ-12. One moment.`

Typing to the task's agent and going into its pane never wait. They also leave the row's words alone: the row keeps
saying `Stopping…` until the stop is done.

A task whose row is not in view (say, one restored from the archive with `Ctrl+Z`) gets the words on the message line
instead: `Restoring PRJ-12 "Fix the build"…`.
