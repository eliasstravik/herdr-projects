# The size of the board

This page explains what changes the size of the board, what does not change it, and how the board hides a change of
size that lasts only a moment.

## In short

The board holds still. Agents that start and end never change its size. The board does not draw a change of size that
lasts less than one second.

If the board looks cut off or half empty, another Herdr window on the same session has the size. Examples are a second
device or a browser tab. To get the size back, type a key in this window.

The board draws itself for the size of the terminal that it runs in. Two things made the screen of the board jump and
jump back. A recording of a real board found both:

1. **The board loads its new version.** This happens when the work of an agent lands. This is the cause that is tied to
   agents.
2. **A second Herdr window on the same session** takes the size and gives it back. This cause is not tied to agents.

## What does not change the size: agents that start and end

Nothing that the board starts opens on the screen of the board, takes its focus, or takes room from its pane.

| Helper | Where it runs |
| --- | --- |
| The agent of a task | A Herdr space of its own (`herdr worktree create`, or `workspace create` outside git), opened with `--no-focus`. |
| Its reviewer | A tab of its own in the space of the task (`tab create --no-focus`, see [tabs](../reference/tabs.md)). The tab closes when the review ends. |
| The Coordinator | A tab of its own in your home space, opened without focus ([the Coordinator](coordinator.md)). The tab closes when the Coordinator stops. |
| The Watcher | A hidden model call. It has no pane. |

A measurement on Herdr 0.9.3 used a real board, agent, reviewer, Watcher and Coordinator. It covered the whole life of a
task (start, review, approval, merge, its space closed), and a Coordinator that started and stopped. Results:

- The terminal of the board kept its size.
- The layout of Herdr for the pane of the board did not change.
- No row of the board moved.

At that time, the reviewer was still a pane split off the pane of the agent. The reviewer now has a tab of its own, and
the same test covers it.

On Herdr, these actions do nothing to a pane in the space that you look at: a space opened without focus, a pane split
off a pane in another space, an agent that starts there, and a space that closes.

A test holds the plugin to this rule (`tests/board_still_sight.rs`):

- The plugin opens every space and every pane with `--no-focus`.
- The plugin never focuses anything while a task runs.
- The plugin never splits or closes the own pane of the board.

Going into an agent on purpose (`Enter` on a task) is the only action that moves the window of Herdr, as before.

## What changes the size

- **Another Herdr window on the same session**: a second device, a second terminal, or a browser tab. Herdr 0.9.3 gives
  every pane the size of the window that was last typed in or resized.
  - With a phone and a desktop on one session, the terminal of the board goes from one size to the other and back. On a
    real board: 141×46 → 138×44 → 141×46 within 0.6 seconds. When a window of phone size took the session: 141×46 →
    47×44. Each time, the board changed shape and changed back.
  - Herdr does this, for every pane, not only for the board. A key typed in the window that you look at gives that
    window the size.
  - It is not tied to agents. A test used a local window and a remote window (`herdr --remote`) of different sizes on one
    scratch session. Spaces, tabs and agents that opened and closed in the background changed the size of no pane,
    whichever window was typed in last.
  - One time, on the real board, the change of 0.6 seconds came in the same half second as the start of a reviewer. The
    log of Herdr shows that the pane of that reviewer started at the size of the other window. Thus that window had the
    session at that moment.
- **A viewer attached to the own terminal of the board** (`herdr terminal attach`, which the web UI of Herdr starts for
  each pane that it shows). It holds that one terminal at its size. The board says so and disconnects it
  ([a board that looks cut off](../reference/board-narrow-window.md#a-board-that-looks-cut-off-or-half-empty)).
- **A pane split off the own pane of the board, or your own window resized.** Only you do these. The plugin never splits
  the pane of the board.

## A size that lasts a moment is not drawn

A board drawn for a different size and, a moment later, for its own size again looks like an error: the side panel goes
and comes back, and every row moves two times. Thus the open board waits:

- **A new size must last one second** before the board draws itself for it. Until then, the screen stays as it was, cut
  to the room that the terminal has. Columns go from the right and rows from the top, so the text box and its hints
  stay. If the old size comes back within the second, nothing changes.
- **Every change waits**, also a change right after another one. The board holds a short change just after a lasting
  one in the same way. While you drag a window, it shows the screen as it was, cut. The board draws itself again once
  a second, not at each step.
- **The board checks the answers of Herdr.** The board asks Herdr what it shows of the pane of the board. Herdr answers
  a moment later.
  - If the answer is for a size that the terminal does not have any more, the board ignores it and asks again.
  - If the answer does not agree with the terminal, the board draws for it only if Herdr gives the same answer again
    one second later.
  - Before this change, the board drew for such an answer until the next answer came, two seconds later.

The cost: after you resize the window yourself, the board follows one second later. While you drag, it follows once a
second.

## A second Herdr window cannot be disconnected

The board takes its terminal back from a viewer attached to it. This does nothing against a second Herdr window on the
whole session: Herdr puts the terminal back at the size of the window last typed in. The log of a real session showed
that the board took its terminal 179 times in twelve minutes, every four seconds, while a second window was on the
session.

Now the board takes the terminal two times. If the sizes are still different, it stops, and says what gives the size
back:

```
 ! Held at 54×24: a key typed in this window frees it. Another herdr window on this session (a second device, a browser tab) has it at its own size, so herdr's own window (142×47) shows the board cut off or half empty
```

The board tries again the next time that the sizes become different after they agreed. The board always keeps the size
of its own pane. No setting turns this off.

## A view that somebody looks through keeps the terminal

The board does not disconnect one kind of viewer: the board open in the terminal view of the web UI of Herdr, in a
browser or on a phone. To disconnect it only started a contest. On herdr-web-ui 0.4.8, the web UI attached again by
itself. The two took the terminal from each other every four seconds, and the board could not be used there.

- **How the board finds such a view:** by its age. The view was opened in the last twenty seconds, or it came back
  within twenty seconds after the board disconnected it. When the board takes a view for such a view, it stays one
  while it runs.
- The board then draws itself for the whole terminal, as that view has it. Its first line says so, for anyone who looks
  at the own window of Herdr meanwhile:

  ```
   Drawn for a remote view of this pane at 119×47 (herdr's web UI has it open): herdr's own window (184×78) shows a cut-out of it until that view is closed
  ```

- When the view closes, the window has its terminal again.
- The board disconnects, as before, a view that nobody looks through (an attach that an old browser left behind). It is
  old, and it does not come back.
- If a view was open before the board started, the board disconnects it one time. The view comes back, and then
  stays.

## The board loads its new version

An open board loads its new version by itself when the file of the program changes under it. This happens in two cases:

- the plugin updates the program ([install and update](../how-to/install-and-update.md));
- you run a program that you build yourself from the repo that you work on with the board. Then it happens each time you
  build it again.

**Before:** the reload stopped the screen in the usual way. That left the full-screen view and printed the board one more
time on the plain screen under it. The new board then cleared the view and drew. A recording on a real terminal, with
one frame every 20 ms, showed this:

1. The whole board one line off for 0.36 seconds.
2. An empty screen for 0.09 seconds.
3. The board again.

**Now:** the old board gives back the modes that it took (keyboard, paste), but it stays in the full-screen view with
its last frame on the screen. The new board is already in that view. It does not enter the view again and does not
clear it. (Herdr clears the view when a program enters it a second time.) It draws its first frame over the old frame.
The same kind of recording showed no frame between the two boards, only the board itself.

If the new board cannot start, the terminal gets its plain screen back. The code is in
`crates/board/src/board/run/term.rs` and `crates/board/src/board/run/start.rs`.

### On the real board, as it happened

On 2026-10-12, these sources showed what happened: the own processes of the board, the history of the checkout, and a
trace of the terminal size of the board, taken every 50 ms while it ran.

- Work landed in the checkout of the plugin at 21:37:34, 21:40:44, 21:48:55, 22:07:47 and 22:10:46.
- The board restarted itself at 21:37:43, 21:40:59, 21:49:00, 22:07:56 and 22:11:07. That is nine to twenty-one seconds
  after each landing, because the board waits until the files stop changing. Every restart followed a merge.
- After five restarts in a row, a board stops restarting. Thus the merges after 22:11 restarted nothing.
- The size trace ran over the third restart. The terminal of the board was 141×46 before and after. It was 140×46 from
  21:49:00.853 to 21:49:01.073: the second when the board restarted, five seconds after the merge.
- A Herdr pane is one column narrower on the plain screen than in the full-screen view (because of the scrollbar). Thus
  that column was the board that left the full-screen view.
- Nothing else changed the size of the terminal in that trace, while agents, reviewers and spaces came and went (ten of
  them).
- The same test ran in a scratch Herdr pane of the same size, 141×46, with the board before the fix. Its code changed
  and the board restarted. The terminal was 140×46 for 0.78 seconds, with the first line of the board blank. Then it
  was 141×46 again.
- With the fix: 141×46 all the time, and the lines of the board were never blank and never moved.

## Related pages

- [The board in a narrow window](../reference/board-narrow-window.md)
- [Tabs](../reference/tabs.md)
- [The Coordinator](coordinator.md)
- [The web UI](web-ui.md)
- [Install and update](../how-to/install-and-update.md)
- [Herdr 0.9.3 facts](../contributing/herdr-0.9.3-facts.md)
