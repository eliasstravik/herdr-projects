# The board's size

**The board holds still.** Agents starting and ending never change its size, and a size change that lasts less than a
second is not drawn. If the board looks cut off or half empty, another Herdr window on the same session (a second
device, a browser tab) has the size: type a key in this window to get it back.

The board is drawn for the size of the terminal it runs in. This page says what changes that size, what does not, and
how the board hides a change that lasts only a moment.

Two things used to make the board's screen jump and jump back. Both were found by recording a real board:

1. **The board loading its new version**, which happens when an agent's work lands. This is the one tied to agents.
2. **A second Herdr window on the same session** taking the size and giving it back. Nothing ties this one to agents.

## What does not change it: agents starting and ending

Nothing the board starts opens on the board's screen, takes its focus, or takes room from its pane:

| Helper | Where it runs |
|---|---|
| A task's agent | a Herdr space of its own (`herdr worktree create`, or `workspace create` outside git), opened with `--no-focus` |
| Its reviewer | a tiled pane in the task's space (`pane split --no-focus`, see [a task's panes](tabs.md)); closed when the review ends |
| The Coordinator | a tab of its own in your home space, opened without focus ([the Coordinator](coordinator.md)); closed when it is stopped |
| The Watcher | a hidden model call: no pane at all |

Measured on Herdr 0.9.3 with a real board, agent, reviewer, Watcher and Coordinator: through a task's whole life (start,
review, approval, merge, its space closed) and a Coordinator started and stopped, the board's terminal kept its size,
Herdr's layout for its pane did not change, and no row of the board moved. The reviewer now uses a split pane beside
the task's agent.

On Herdr, a space opened without focus, a pane split off a pane in another space, an agent starting there, and a space
closing do nothing to a pane in the space you look at.

A test holds the plugin to it (`tests/board_still_sight.rs`): every space and pane it opens is opened with `--no-focus`,
it never focuses anything while a task runs, and it never splits or closes the board's own pane. Going into an agent on
purpose (`Enter` on a task) is the one thing that moves Herdr's window, as before.

## What does change it

- **Another Herdr window on the same session**: a second device, a second terminal, a browser tab. Herdr 0.9.3 gives
  every pane the size of the window that was last typed in or resized. With a phone and a desktop on one session, the
  board's terminal goes back and forth between the two sizes: seen on a real board as 141×46 → 138×44 → 141×46 within
  0.6 seconds, and as 141×46 → 47×44 when a phone-sized window took the session. Each time the board changed shape and
  changed back.
  - It is Herdr's doing, for every pane, not just the board's: a key typed in the window you look at gives that window
    the size.
  - It is not tied to agents. With a local and a remote window (`herdr --remote`) of different sizes on one scratch
    session, spaces, tabs and agents opened and closed in the background changed no pane's size, whichever window was
    typed in last. (Once, on the real board, the 0.6-second change came in the same half second as a reviewer starting.
    Herdr's log shows that reviewer's pane spawned at the other window's size, so that window had the session at that
    moment.)
- **A viewer attached to the board's own terminal** (`herdr terminal attach`, which Herdr's web UI starts for each pane
  it shows): it holds that one terminal at its size. [The board](board.md) says so and disconnects it.
- **A pane split off the board's own pane, or your own window resized.** Only by you: the plugin never does the first.

## A size that lasts a moment is not drawn

A board laid out for another size and, a moment later, for its own again looks like a glitch: the side panel goes and
comes back, every row moves twice. So the open board waits:

- **A new size has to last one second** before the board lays itself out for it. Until then the screen stays exactly as
  it was, cut to the room the terminal has: columns go from the right and rows from the top, so the text box and its
  hints stay. A size that is back within the second leaves nothing changed.
- **Every change waits**, also one right after another: a short change just after a lasting one is held the same way. A
  window you drag shows the screen as it was, cut, and is laid out again once a second, not at each step.
- **Herdr's answers are checked.** Herdr is asked what it shows of the board's pane, and answers a moment later. An
  answer given for a size the terminal no longer has is put aside and Herdr is asked again. An answer that disagrees with
  the terminal is drawn for only when Herdr says the same again a second later. (Before, such an answer was drawn for
  until the next one came, two seconds on.)

The cost: after you resize the window yourself, the board follows a second later, and while you drag, once a second.

## A second Herdr window cannot be disconnected

The board takes its terminal back from a viewer attached to it. Against a second Herdr window on the whole session,
that changes nothing: Herdr puts the terminal back at the size of the window last typed in. A real session's log showed
the board taking its terminal 179 times in twelve minutes, every four seconds, while a second window was on the session.

Now it takes the terminal twice. If the sizes are still apart, it stops, and says what gives the size back:

```
 ! Held at 54×24: a key typed in this window frees it. Another herdr window on this session (a second device, a browser tab) has it at its own size, so herdr's own window (142×47) shows the board cut off or half empty
```

It tries again the next time the sizes come apart after having agreed. The board keeping the size of its own pane is
fixed behaviour: there is no setting that turns it off.

## A view somebody is looking through keeps the terminal

One kind of viewer is not disconnected: the board opened in the terminal view of Herdr's web UI, in a browser or on a
phone. Disconnecting it only started a contest (seen on herdr-web-ui 0.4.8: the web UI reattached by itself, and the two
took the terminal from each other every four seconds, so the board could not be used there).

- **How it is recognised:** by its age. It was opened in the last twenty seconds, or it came back within twenty seconds
  of being disconnected. Once taken for one, it stays one for as long as it runs.
- The board is then drawn for the whole terminal, as that view has it, and its first line says so for whoever looks at
  Herdr's own window meanwhile:

  ```
   Drawn for a remote view of this pane at 119×47 (herdr's web UI has it open): herdr's own window (184×78) shows a cut-out of it until that view is closed
  ```

- When the view is closed, the window has its terminal again.
- A view nobody looks through (an attach an old browser left behind) is disconnected as before: it is old, and it does
  not come back.
- A view that was open before the board started is disconnected once, comes back, and then stays.

## The board loading its new version

An open board loads its new version by itself when the program's file changes under it: when the plugin updates the
program ([installing](install.md)), or, if you run a program you build yourself from the repo you work on with the
board, every time you build it again.

**Before:** the reload stopped the screen the usual way. That leaves the full-screen view and prints the board once more
on the plain screen under it; the new board then cleared the view and drew. Recorded on a real terminal, a frame every
20 ms: the whole board one line off for 0.36 seconds, then an empty screen for 0.09 seconds, then the board again.

**Now:** the old board gives back the modes it took (keyboard, paste) but stays in the full-screen view with its last
frame up. The new board, already in that view, neither enters it again nor clears it (Herdr clears the view on entering
it a second time), and draws its first frame over the old one. Recorded the same way: no frame between the two boards
but the board itself. If the new board cannot start, the terminal gets its plain screen back.
(`crates/board/src/board/run/term.rs`, `crates/board/src/board/run/start.rs`.)

### On the real board, as it happened

2026-10-12, read from the board's own processes, the checkout's history, and a trace of the board's terminal size taken
every 50 ms while it ran:

- Work landed in the plugin's checkout at 21:37:34, 21:40:44, 21:48:55, 22:07:47 and 22:10:46. The board restarted itself
  at 21:37:43, 21:40:59, 21:49:00, 22:07:56 and 22:11:07, nine to twenty-one seconds after each (it waits until the files
  stop changing). Every restart followed a merge. (After five restarts in a row a board stops restarting, so the merges
  after 22:11 restarted nothing.)
- The size trace ran over the third of these. The board's terminal, 141×46 before and after, was 140×46 from
  21:49:00.853 to 21:49:01.073: the second the board restarted, five seconds after the merge. A Herdr pane is one column
  narrower on the plain screen than in the full-screen view (the scrollbar), so that column is the board leaving the
  full-screen view. Nothing else changed the terminal's size in that trace while agents, reviewers and spaces came and
  went (ten of them).
- The same in a scratch Herdr pane of the same size, 141×46, with the board as it was: its code changed, the board
  restarted, the terminal was 140×46 for 0.78 seconds with the board's first line blank, then 141×46 again. With the fix:
  141×46 throughout, and the board's lines never blank or moved.
