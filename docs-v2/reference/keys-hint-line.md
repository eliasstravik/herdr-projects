# The hint line

This page tells what the hint line at the bottom of the board shows, and how it becomes shorter in a narrow window.

## What the hint line shows

The bottom line shows only what matters now. The main hint is first. It shows a maximum of four hints for the selected
task. The hint line is not a list of all keys: `?` lists every key.

- **A hint says what the key does, and to what.** "This task" is the selected task: `enter open this task's agent`,
  `tab put an agent on this task`, `ctrl+l approve and merge this task`. The line says one time whose agent it is.
  After that, it says `the agent` (`space talk to the agent · ctrl+x stop the agent`). The hint line and the `?` menu
  use the same words for the same action.
- **The hint line names a key only where the key does something.**
  - `esc` shows with the thing that it goes back from: `esc back to the list`, `esc clear the filter`, or
    `esc back to the board` in the archive. It does not show where it only tells how the board closes.
  - `↑↓ scroll the task` shows on the whole task only while the task is longer than the window.
  - `alt+c` shows only while there is a message to show or hide.
  - `ctrl+x` says what it does here: `stop the agent`, `remove this task`, or `put it back in to do` on a task that you
    marked as your own work.
- **The hint line does not repeat what the screen says already.** The grey words in the text box tell who gets your
  message (`Send message to Agent`, `Send message to Coordinator`). Thus the hint line does not tell it.
- **While you type**, the hint line tells where `Enter` sends your words. It names the task by its tag, because your
  words are locked to that task while the list can show other tasks. Examples: `enter send to the Coordinator`
  (`… on vm-a` for the Coordinator of another machine), `enter send to HSW-34's agent`,
  `enter send, and start an agent on HSW-34`, `enter add it as a task`. The edge of the text box names no one.

## Examples

| Where | The hint line |
|---|---|
| A task nobody started | `tab put an agent on this task · space talk about this task · ? all keys` |
| An agent at work | `enter open this task's agent · space talk to the agent · ctrl+x stop the agent · ? all keys` |
| An agent asks you something | `space answer this task's agent · enter open the agent · ? all keys` |
| Ready to approve | `ctrl+l approve and merge this task · ctrl+r send the review to the agent · space talk to the agent · ? all keys` |
| Something went wrong | `ctrl+r tell this task's agent to fix it · ctrl+l retry as it is · space talk to the agent · ? all keys` |
| Stopped without a summary | `ctrl+r tell this task's agent to continue · ctrl+l use its work as it is · enter open the agent · ? all keys` |
| Talking to a task's agent | `enter open this task's agent · ctrl+x stop the agent · space back to the Coordinator` |
| Typing to the Coordinator | `enter send to the Coordinator · tab send, and put an agent on what it adds · shift+enter new line · esc put away` |
| The whole task | `… · ↑↓ scroll the task · esc back to the list` |
| The archive | `ctrl+a restore this task · ctrl+x remove this task · esc back to the board · ? all keys` |
| An automation's row | `enter open this automation · space turn this automation off · tab fire it now · ctrl+x remove it · ctrl+e its settings · ? all keys` (off: `space turn this automation on`, and no `tab`, which would fire nothing) |
| One automation, whole | `↑↓ pick a firing · enter go to its task · space turn this automation off · tab fire it now · esc back to the list` (`enter` only on a firing that made a task) |
| Find | `enter keep the filter · esc cancel` |
| The `?` menu | `↑↓ pick a key · enter press it · esc close the menu` |
| Words on their way to another machine | `ctrl+x stop sending this · enter take the words back to change · ? all keys` |

## In a narrow window

In a narrow window, the hint line never goes off the screen. The board removes full hints. It does not cut a hint.

- The first hint (the main hint) and the last hint (`? all keys`, or the way back) stay.
- Between them, `alt+c hide the latest message` goes first. Then the other hints go, from the last hint back.
  `alt+c show the latest message` stays longer than those hints, because it is how you see a reply.
- While you type, `shift+enter new line` goes first, and `esc put away` stays.
- If there is space for only one hint, that hint is the main hint. The exception is while you talk to the agent of a
  task. Then `space back to the Coordinator` stays, because it is the only way back.
- If the window is too narrow for that one hint, the board shows a shorter form of the hint. It does not cut it.
  Examples: `enter send to HSW-34`, `ctrl+l merge this task`, and `its agent` in place of `this task's agent`. This
  works down to approximately 34 columns.
- The hint for a draft continues to name who gets the words while that fits (`enter send to HSW-34`,
  `enter send to the Coordinator`, `enter send to vm-a`). Only in a narrower window is it `enter send`.

## Related pages

- [Keys](keys.md)
- [Add your own keys](../how-to/add-your-own-keys.md)
- [Talk to a Coordinator](../how-to/talk-to-a-coordinator.md)
