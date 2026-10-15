# Keys, questions and the ? menu

**Press `?` on the board to see every key.** The tables at the end of this page list them all too. One rule makes them
easy to guess:

- **`Ctrl` does something** (approve, stop, archive, …).
- **`Alt` only shows or hides a part of the screen**, and never changes anything.
- **Plain keys** (`Enter`, `Tab`, `Space`, `Esc`, the arrows) move around and talk.

No other chord does anything, and no key of an earlier version works as a second way to press one of these.

Two keys bend the rule. `Ctrl+F` (find) only changes which tasks the list shows, so by the rule it would be an `Alt` key;
it is the key every program finds with, so it stays. `Alt+M` only shows other machines, but while one machine is shown by
itself, what you type goes to that machine's Coordinator: the chat says so.

## Keys and the text box

Apart from `Enter`, `Tab`, `Esc` and `Ctrl+C`, a key acts on the board only while the text box is empty. With a draft in
it, the keys belong to the draft (`Ctrl+A` is "start of the line" there, not "archive").

The one exception is the two keys that scroll the chat's message, `Alt+K` and `Alt+J`: they only show more of it, so they
work while you type and leave the draft alone.

## The `?` menu

It lists the same three groups as the tables below, each line with its key. A `Ctrl` or `Alt` line can also be picked
there, which does what the key does. So a terminal that does not pass `Alt` on (a Mac terminal without "Option as Meta",
some phone keyboards) loses nothing.

In the terminal view of Herdr's web UI, the `Alt` keys and `Shift+Enter` arrive in the old byte form and are taken as
such, and the Enter a phone's input line adds after `?`, `y`, `n` or space is dropped ([the web UI](web-ui.md)).

## Questions: the board asks before what cannot be undone

Only merging and removing a task ask. Nothing else does.

```
 Merge HSW-18 "Add an undo for clear" (PR#18) into main?
 A merge cannot be undone.
 y = merge · n = do not merge
```

- **`y` says yes. `n` or `Esc` says no**, in upper or lower case. `Enter` never confirms, and no other key answers: on
  any other key the box says `press y or n`.
- **The question says what it does.** A merge names the task, where it lands and that it cannot be undone. For another
  machine's task, it says where it lands there: `into main on otherbox`. After a failed review, it says what the review
  found, and its yes reads `y = merge anyway`.
- **A remove** shows the task's whole title and what is really lost: the task with its description and reviews, and its
  worktree only where it has one (work in it that is not saved or not merged is kept). Its keys: `y = remove · n = keep`.
- **Where it shows:** in a small box at the bottom of the board, in the text box's place, its closing line on the
  window's last line, as the chat's box sits on its hints. A question longer than the window wraps, and its keys are
  never cut: in a short window its detail gives way first, then its rules. There is no hint line meanwhile: the box takes
  its row.
- On the web Board neither answer to a merge or a remove is the filled button.

A result stays on screen until your next key press. An error stays until `Esc` dismisses it, with `(esc dismisses)` after
it.

### Work whose review failed is never merged by a plain yes

It is not under `ready to approve` but under `needs you`, as `Review failed · 1 critical, 1 major`. Its hints offer
sending the review back first, then `merge this task anyway`.

The guard is in the approve action itself, not only in the question: `herdr-projects approve <id>` (and so the
Coordinator) refuses such work and says what the review found. `herdr-projects approve <id> --anyway` merges it all the
same.

## Closing the board

**`Ctrl+C` twice**, as in Claude Code. The first shows `Press ctrl+c again to close the board` on the bottom line;
`Ctrl+C` again while it shows (two seconds) closes, even with a draft in the text box. Any other key, or the wait running
out, takes the hint away, and that key does what it always does.

`Esc` with nothing left to step back from closes nothing: its line names the key that does (`ctrl+c closes the board`),
and `Ctrl+C` while it shows closes. In the popup, that last `Esc` closes the board, and the hint line says so
(`esc close`).

## The key that opens the board

It is Herdr's, not the board's: a line in Herdr's `config.toml`, offered at the [first run](install.md) as `prefix+a`.
The board never takes a key Herdr binds itself (`prefix+w` is Herdr's workspace picker) or one your config already binds:
it refuses it and says what the key does today.

Pressed again, the board key takes you back: from the board in its tab, to the pane you came from; with `board.open` set
to `popup`, it closes the popup ([the board](board.md)). Herdr gives a popup every key, so there the board reads that key
itself, from Herdr's `config.toml`: Herdr's prefix key and then the key.

## The settings page owns its keys

While a task's settings are open (`Ctrl+E`), only that page's keys act: `↑` `↓`, `Enter`, `Ctrl+U`, `Esc` (or `Ctrl+E`
again), and `Ctrl+C` to close the board. No key of the list archives, merges, pins or removes the task from there. The
hint line is the page's own: `↑ ↓ move · enter change · ctrl+u put back · esc back to the task`, and while a value is
typed, `enter save this value · esc leave it as it is`.

## The hint line

The bottom line shows only what matters right now, the main thing first, at most four hints for the selected task. `?`
lists every key; the hint line is not a list.

- **A hint says what the key does, and to what.** "This task" is the selected one: `enter open this task's agent`,
  `tab put an agent on this task`, `ctrl+l approve and merge this task`. Whose agent is said once on the line; after that
  it is `the agent` (`space talk to the agent · ctrl+x stop the agent`). The same action has the same words on the hint
  line and in the `?` menu.
- **A key is named only where it does something.** `esc` shows with what it goes back from (`esc back to the list`,
  `esc clear the filter`, `esc back to the board` in the archive), not where it would only show how the board closes.
  `↑↓ scroll the task` shows on the whole task only while it is longer than the window. `alt+c` shows only while there is
  a message to show or hide. `ctrl+x` says what it would do here: `stop the agent`, `remove this task`, or, on a task you
  marked as your own work, `put it back in to do`.
- **What the screen already says is not said again.** The text box's own grey words say who gets your message
  (`Send message to Agent`, `Send message to Coordinator`), so the line does not.
- **While you type**, it says where `Enter` sends your words, and names the task by its tag, since the words are locked to
  it while the list may show others: `enter send to the Coordinator` (`… on vm-a` for another machine's),
  `enter send to HSW-34's agent`, `enter send, and start an agent on HSW-34`, `enter add it as a task`. The text box's edge
  names no one.

What it shows, for example:

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

**In a narrow window the line never runs off the screen: whole hints give way.**

- The first hint (the main thing) and the last one (`? all keys`, or the way back) stay.
- Between them, `alt+c hide the latest message` goes first, then the others from the last back. `alt+c show the latest message` outlasts those, because it is how you see a reply.
- While you type, `shift+enter new line` goes first and `esc put away` stays.
- With room for only one hint, it is the main thing, except while you talk to a task's agent: there
  `space back to the Coordinator` stays, the one way back.
- Too narrow even for that one hint, it is said shorter, not cut: `enter send to HSW-34`, `ctrl+l merge this task`,
  `its agent` for `this task's agent`. That holds down to about 34 columns. A draft's hint keeps naming who gets the words
  as long as that fits (`enter send to HSW-34`, `enter send to the Coordinator`, `enter send to vm-a`); only narrower is
  it `enter send`.

Restarting the Coordinator, starting a new one, and going into its whole chat are not on the board: see
[the Coordinator](coordinator.md).

## All the keys

<!-- made:keys Made by `herdr-projects docs` from crates/flow/src/board/keys.rs. Do not edit between these two lines. -->
**Plain keys**

| Key | Does |
|---|---|
| `↑` `↓` | move between tasks and their headings, and on to the automations under them |
| `Pgup` `Pgdn` | a page up or down: ten tasks in the list, a page of text in the whole task |
| `Enter` | send what you typed; with nothing typed, open the task's agent: its real herdr pane. The board keeps running in its own tab and is there as you left it; your key goes back to it (as a popup it closes, and your key opens it again where it was). On an automation's row: open the automation, whole ([automations.md](automations.md)). On a heading (a machine, a repo, a status group): fold it to its one line, which says how many tasks it holds, and unfold it; its caret says which it is, `▾` open, `▸` folded ([list-headings.md](list-headings.md)) |
| `Space` | flip who you talk to: the selected task's agent, and back to the Coordinator. The chat says who. Nothing else changes it: while it is a task's agent, moving with `↑` `↓` makes it the agent of the task you moved to. On a task with no agent, your message puts one on it. On an automation's row: turn the automation on or off |
| `Tab` | put an agent on the task. With text typed for the Coordinator: send it, and every task it adds from that message gets an agent at once, with nothing more to confirm (a message that adds no task delegates nothing). On an automation's row: fire the automation now |
| `Esc` | back, one step at a time, and nothing else: find is left, a draft is put away, then the whole task, the filter and the archive close, then the chat's latest message is hidden, as `Alt+C` hides it. With none of those left it shows the same hint as `Ctrl+C`; the board as a popup (`board.open`) closes on it. It never shows the chat and never changes who you talk to |
| `?` | the menu: every key in these three groups, each line with its key. A ctrl or alt line can be picked with `↑` `↓` and `Enter`, which does what its key does. It opens with no line picked. Under the keys, what a row shows: each mark with its word, the numbers on the right, and what the colour of a review tag and of a pull request say. Every line is whole (a long one wraps); where the window is too short for all of it the last line says how many lines are below, and `↓` on the last key scrolls on down, `↑` on the first scrolls up |
| `y` `n` | answer a question the board asks: `y` yes, `n` or `Esc` no. Only while a question is up; `Enter` never confirms |

**`Ctrl`: does something**

| Key | Does |
|---|---|
| `Ctrl+L` | yes, to whatever the task waits for: approve the merge (it asks first), keep the selected one of several attempts (it asks first), say yes to the agent, retry the step that went wrong as it stands, or take work that stopped without a summary, or at a limit, as it is; on a task whose agent is at rest while its row still says "Finishing", finish it with what its agent said last |
| `Ctrl+R` | send it back: whatever went wrong (a failed check, a conflict, an error, a review with findings) goes to the task's agent to fix; a stopped agent is told to continue |
| `Ctrl+N` | on a task done several times at once ([attempts.md](attempts.md)): move to its next attempt, in the task panel. `Ctrl+L` keeps the selected one, `Enter` goes into its tab |
| `Ctrl+X` | stop the agent, when one is running (working, starting, checking, under review, or waiting for your answer); the selection stays where you were, on the next row when the stopped task moves to another group. On a task with no agent running (idle, never started, stopped, finished, archived) it asks to remove the task instead: `y` removes it, its worktree too. A task you marked as your own work in progress has no agent to stop: there the key puts it back in to do. A stop never goes on to that question by itself: removing takes a second `Ctrl+X`, and a `y`. On an automation's row: remove the automation (it asks first; the tasks it made stay) |
| `Ctrl+A` | archive the selected task, or restore it from the archive |
| `Ctrl+P` | pin the selected task to the top, or unpin it |
| `Ctrl+E` `Ctrl+U` | the selected task's settings, full screen: every work setting by its name and its value. `↑` `↓` move to a row; `Enter` changes it (a switch or a choice goes to its next value, anything else is typed and saved with `Enter`); rows set for this task only are marked `*` (where any other value comes from is `herdr-projects show <id>`'s). `Ctrl+U` puts the selected row back to what it would be without this task's own answer. `Esc` or `Ctrl+E` again goes back ([settings.md](settings.md)). On an automation's row: the automation's own settings, the same page |
| `Ctrl+Z` | undo: bring back a draft that was put away, or take the last archiving back |
| `Ctrl+F` | find: type, and the list shows only the tasks that match, archived ones too. `Enter` keeps the filter, `Esc` leaves find; `Esc` on the list clears a filter that is kept |
| `Ctrl+K` | only while a remote view holds the board's terminal at another size than herdr shows it (the line at the top says so): end that view, so the board has its own size again ([board-size.md](board-size.md)) |
| `Ctrl+C` | twice: close the board, with text typed too: the draft is kept and is back when the board reopens |

**`Alt`: only shows or hides**

| Key | Shows or hides |
|---|---|
| `Alt+C` | the chat's latest message, the Coordinator's or a task agent's. The one key for it, both ways |
| `Alt+K` `Alt+J` | more of a chat message that is longer than its box: `Alt+K` scrolls it up, `Alt+J` down, half a box at a time, also while you type. Up opens the box to the conversation before the newest message, each press further up; it stays open, whatever comes meanwhile, until `Alt+J` at its end goes back to the newest message (`Alt+C` hides and shows the whole chat; talking to another starts it at the newest too). The box says where there is more, and names the key (`alt+k more ↑`, `alt+j more ↓`). The mouse wheel does the same with the pointer over the chat ([mouse.md](mouse.md)) |
| `Alt+P` | the side panel: the selected task or automation, beside the list |
| `Alt+T` | the whole task (on an automation's row: the whole automation), full screen, and back to the list |
| `Alt+A` | the archive, and back to the board |
| `Alt+S` | the list grouped by repo, each group under its repo's name (a row's tag never names a repo), or by status |
| `Alt+D` | every done task, or only the latest |
| `Alt+M` | other machines: this machine only, every machine, then each machine by itself |

**The text box's own** (not a key of the board, and not in the `?` menu)

| Key | Does |
|---|---|
| `Shift+Enter` | a new line in what you are typing. Like the cursor keys and backspace, it is the text box's own editing key, not a key of the board |
<!-- /made:keys -->
