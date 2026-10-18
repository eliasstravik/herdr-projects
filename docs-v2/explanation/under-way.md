# What a key shows at once

This page explains what the row of a task shows after you press a key that acts on the task, while the work is under
way, and when it ends or fails.

## The row shows the action at once

A key that acts on a task shows the action at once, also when the work takes some time. Examples of such work: the
board sets up a worktree, starts an agent, merges, or waits for another machine to answer.

- The row gets the working mark and shows what happens, in the working colour.
- The task panel shows the same text under Status, as `Now`.

| Key | The row shows |
| --- | --- |
| `Tab`, or a first reply to a task with no agent | `Starting…` |
| `Ctrl+X` on a running task | `Stopping…` |
| `Ctrl+X` then `y` on a task that does not run | `Removing…` |
| `Ctrl+L` | `Merging…` (after the `y`), `Approving…`, `Trying again…` or `Finishing as it is…`. The text depends on what the key says yes to. |
| `Ctrl+R` | `Sending back…` |
| `Ctrl+A`, `Ctrl+Z` | `Archiving…`, `Restoring…` |
| `Ctrl+P` | `Pinning…`, `Unpinning…` |
| `y` when you pick an attempt | `Keeping attempt B…` |
| `Enter` with nothing typed | `Opening…` (until its pane opens) |
| `Space`, `Tab`, `Ctrl+X` on an automation | `Turning on…`, `Turning off…`, `Firing…`, `Removing…` |

## When the work ends

The words stay until the board reads the task again after the work ended. Then the row shows the real state of the
task.

A read that started before the work ended does not count. Thus the row never changes back to the old state for a
moment.

For a task of another machine, the words stay until one of these happens:

- that machine shows that the task changed;
- that machine answered two times (at most 15 seconds).

## When the work fails

The row shows the failure in red, with the error mark: `Could not stop: …` and the reason.

- The task panel shows the same text under `Now`.
- The message line shows it until you press `Esc`.
- The row goes back to normal after ten seconds, or when `Esc` dismisses the error.

## When you press the key again

The key does nothing while the work is under way. Thus nothing starts two times. The message line shows
`Still starting PRJ-12. One moment.`

Two actions never wait: typing to the agent of the task, and going into its pane. They also do not change the words of
the row. The row continues to show `Stopping…` until the stop is done.

## A task that is not in view

If the row of the task is not in view, the message line shows the words. An example is a task that you restore from the
archive with `Ctrl+Z`. The message line then shows `Restoring PRJ-12 "Fix the build"…`.

## Related pages

- [The rows of the board](../reference/board-rows.md)
- [Marks and colours on a row](../reference/board-marks-and-colours.md)
- [Keys](../reference/keys.md)
- [The task panel](../reference/task-panel.md)
- [Automations in the list](../reference/board-automations.md)
