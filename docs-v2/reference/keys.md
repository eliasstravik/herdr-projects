# Keys

This page lists the keys of the board and the rules that they obey. It also tells you how the board asks a question,
how to close the board, and which key opens the board.

## The rule for the keys

Press `?` on the board to see every key. The table in [All the keys](#all-the-keys) also lists every key. One rule
makes the keys easy to guess:

- **`Ctrl` keys do an action** (approve, stop, archive, and more).
- **`Alt` keys only show or hide a part of the screen.** They never change anything.
- **Plain keys** (`Enter`, `Tab`, `Space`, `Esc` and the arrow keys) move on the board and send your words.

No other chord does an action. No key of an earlier version works as a second key for one of these actions.

Two keys do not obey the rule fully:

- `Ctrl+F` (find) only changes which tasks the list shows. By the rule, it is an `Alt` key. It stays `Ctrl+F` because
  every program uses that key to find.
- `Alt+M` only shows other machines. But while the board shows one machine alone, the words that you type go to the
  Coordinator of that machine. The chat tells you this.

## Keys and the text box

Most keys act on the board only while the text box is empty. `Enter`, `Tab`, `Esc` and `Ctrl+C` always act on the
board. While the text box holds a draft, the other keys edit the draft. For example, `Ctrl+A` moves to the start of the
line in the draft. It does not archive.

There is one more exception: `Alt+K` and `Alt+J`, the two keys that scroll the message in the chat. They only show
more of the message. Thus they work while you type, and they do not change the draft.

## The `?` menu

The `?` menu lists the same three groups as the tables in [All the keys](#all-the-keys). Each line has its key. You
can pick a `Ctrl` line or an `Alt` line in the menu. The board then does what the key does. Thus a terminal that does
not send `Alt` to the board loses nothing. Examples are a Mac terminal without "Option as Meta", and some phone
keyboards.

In the terminal view of the Herdr web UI, the `Alt` keys and `Shift+Enter` come in the old byte form. The board reads
them in that form. A phone input line adds an Enter after `?`, `y`, `n` or space. The board ignores that Enter. More
information: [the web UI](web-ui.md).

## Questions

The board asks a question before an action that you cannot undo. Only two actions ask: a merge and the removal of a
task. No other action asks.

```
 Merge HSW-18 "Add an undo for clear" (PR#18) into main?
 A merge cannot be undone.
 y = merge · n = do not merge
```

- **`y` says yes. `n` or `Esc` says no.** Upper case and lower case both work. `Enter` never confirms. No other key
  answers. If you press a different key, the box shows `press y or n`.
- **The question says what the action does.** A merge question names the task, the branch where the work lands, and
  that you cannot undo the merge. For a task on another machine, it names the branch on that machine:
  `into main on otherbox`. After a failed review, the question tells what the review found, and its yes is
  `y = merge anyway`.
- **A remove question** shows the full title of the task and the data that you really lose: the task, its description
  and its reviews. It also names the worktree, but only if the task has one. Work in the worktree that is not saved or
  not merged is kept. The keys are `y = remove · n = keep`.
- **The question box** shows in a small box at the bottom of the board, in the place of the text box. Its last line is
  on the last line of the window, as the chat box sits on its hints. A question that is longer than the window wraps.
  Its keys are never cut. In a short window, the detail goes first, then the rules. While the question shows, there is
  no hint line, because the box uses that row.
- On the web board, neither answer to a merge or a remove question is the filled button.

A result stays on the screen until you press the next key. An error stays until you press `Esc`. The error shows
`(esc dismisses)` after it.

### Work with a failed review

A plain yes never merges work whose review failed. That task is not under `ready to approve`. It is under `needs you`,
as `Review failed · 1 critical, 1 major`. Its hints first offer to send the review back to the agent. Then they offer
`merge this task anyway`.

The guard is in the approve action itself, not only in the question. `herdr-projects approve <id>` refuses this work
and tells what the review found. Thus the Coordinator also cannot merge it. `herdr-projects approve <id> --anyway`
merges the work all the same.

## Close the board

Press **`Ctrl+C` two times**, as in Claude Code.

- The first `Ctrl+C` shows `Press ctrl+c again to close the board` on the bottom line.
- A second `Ctrl+C` while that line shows (two seconds) closes the board. This is also true when the text box holds a
  draft.
- If you press a different key, or the two seconds end, the hint goes away. That other key does its usual action.

`Esc` with nothing left to go back from closes nothing. Its line names the key that closes the board
(`ctrl+c closes the board`). `Ctrl+C` while that line shows closes the board. In the popup, that last `Esc` closes the
board, and the hint line tells you so (`esc close`).

## The key that opens the board

The key that opens the board is a key of Herdr, not of the board. It is a line in the `config.toml` file of Herdr. The
[first run](../how-to/install-and-update.md) offers it as `prefix+a`. The board never takes a key that Herdr binds
itself (`prefix+w` is the workspace picker of Herdr). It also never takes a key that your config binds already. It
refuses that key and tells you what the key does now.

If you press the board key again, it takes you back:

- From the board in its tab, it takes you to the pane that you came from.
- If `board.open` is `popup`, it closes the popup. More information:
  [the board](board-header-and-list.md).

Herdr gives a popup every key. Thus in a popup, the board reads that key itself, from the `config.toml` file of Herdr:
the prefix key of Herdr and then the key.

## Keys on the settings page

While the settings of a task are open (`Ctrl+E`), only the keys of that page act:

- `↑` and `↓`
- `Enter`
- `Ctrl+U`
- `Esc`, or `Ctrl+E` again
- `Ctrl+C`, to close the board

No key of the list archives, merges, pins or removes the task from that page. The page has its own hint line:
`↑ ↓ move · enter change · ctrl+u put back · esc back to the task`. While you type a value, the hint line is
`enter save this value · esc leave it as it is`. More information: [the task panel](task-panel.md#the-settings-page).

## The hint line

The bottom line of the board shows hints for the keys that matter now. The rules and examples are on
[the hint line](keys-hint-line.md).

The board does not show these actions: restart the Coordinator, start a new Coordinator, and go into the full chat of
the Coordinator. Refer to [talk to a Coordinator](../how-to/talk-to-a-coordinator.md).

## Your own keys

You can move a key to a chord of your own with the setting `board.keys`. The steps and the rules are in
[add your own keys](../how-to/add-your-own-keys.md). The tables below always show the chords that the board has when
you install it.

## All the keys

Note: This table is a snapshot of the table that the code makes in [`docs/keys.md`](../../docs/keys.md). If the two
are different, `docs/keys.md` is correct. The links in the table go to the pages in `docs-v2/` that hold the same topics.

**Plain keys**

| Key | Does |
|---|---|
| `↑` `↓` | move between tasks and their headings, and on to the automations under them |
| `Pgup` `Pgdn` | a page up or down: ten tasks in the list, a page of text in the whole task |
| `Enter` | send what you typed; with nothing typed, open the task's agent: its real herdr pane. The board keeps running in its own tab and is there as you left it; your key goes back to it (as a popup it closes, and your key opens it again where it was). On an automation's row: open the automation, whole ([automations.md](automations.md)). On a heading (a machine, a repo, a status group): fold it to its one line, which says how many tasks it holds, and unfold it; its caret says which it is, `▾` open, `▸` folded ([list-headings.md](list-headings.md)) |
| `Space` | flip who you talk to: the selected task's agent, and back to the Coordinator. The chat says who. Nothing else changes it: while it is a task's agent, moving with `↑` `↓` makes it the agent of the task you moved to. On a task with no agent, your message puts one on it. On a machine's or a repo's heading: that machine's or repo's own coordinator. On an automation's row: turn the automation on or off |
| `Tab` | put an agent on the task. With text typed for the Coordinator: send it, and every task it adds from that message gets an agent at once, with nothing more to confirm (a message that adds no task delegates nothing). On an automation's row: fire the automation now |
| `Esc` | back, one step at a time, and nothing else: find is left, a draft is put away, then the whole task, the filter and the archive close, then the chat's latest message is hidden, as `Alt+C` hides it. With none of those left it shows the same hint as `Ctrl+C`; the board as a popup (`board.open`) closes on it. It never shows the chat and never changes who you talk to |
| `?` | the menu: every key in these three groups, each line with its key. A ctrl or alt line can be picked with `↑` `↓` and `Enter`, which does what its key does. It opens with no line picked. Under the keys, what a row shows: each mark with its word, the numbers on the right, and what the colour of a review tag and of a pull request say. Every line is whole (a long one wraps); where the window is too short for all of it the last line says how many lines are below, and `↓` on the last key scrolls on down, `↑` on the first scrolls up |
| `y` `n` | answer a question the board asks: `y` yes, `n` or `Esc` no. Only while a question is up; `Enter` never confirms |

**`Ctrl`: does something**

| Key | Does |
|---|---|
| `Ctrl+L` | yes, to whatever the task waits for: approve the merge (it asks first), keep the selected one of several attempts (it asks first), say yes to the agent, retry the step that went wrong as it stands, or take work that stopped without a summary, or at a limit, as it is; on a task whose agent is at rest while its row still says "Finishing", finish it with what its agent said last |
| `Ctrl+R` | send it back: whatever went wrong (a failed check, a conflict, an error, a review with findings) goes to the task's agent to fix; a stopped agent is told to continue |
| `Ctrl+N` | on a task done several times at once ([attempts.md](attempts-settings.md)): move to its next attempt, in the task panel. `Ctrl+L` keeps the selected one, `Enter` goes into its tab |
| `Ctrl+X` | stop the agent, when one is running (working, starting, checking, under review, or waiting for your answer); the selection stays where you were, on the next row when the stopped task moves to another group. On a task with no agent running (idle, never started, stopped, finished, archived) it asks to remove the task instead: `y` removes it, its worktree too. A task you marked as your own work in progress has no agent to stop: there the key puts it back in to do. A stop never goes on to that question by itself: removing takes a second `Ctrl+X`, and a `y`. On an automation's row: remove the automation (it asks first; the tasks it made stay) |
| `Ctrl+A` | archive the selected task, or restore it from the archive |
| `Ctrl+P` | pin the selected task to the top, or unpin it |
| `Ctrl+E` `Ctrl+U` | the selected task's settings, full screen: every work setting by its name and its value. `↑` `↓` move to a row; `Enter` changes it (a switch or a choice goes to its next value, anything else is typed and saved with `Enter`); rows set for this task only are marked `*` (where any other value comes from is `herdr-projects show <id>`'s). `Ctrl+U` puts the selected row back to what it would be without this task's own answer. `Esc` or `Ctrl+E` again goes back ([settings.md](settings.md)). On an automation's row: the automation's own settings, the same page |
| `Ctrl+Z` | undo: bring back a draft that was put away, or take the last archiving back |
| `Ctrl+F` | find: type, and the list shows only the tasks that match, archived ones too. `Enter` keeps the filter, `Esc` leaves find; `Esc` on the list clears a filter that is kept |
| `Ctrl+K` | only while a remote view holds the board's terminal at another size than herdr shows it (the line at the top says so): end that view, so the board has its own size again ([board-size.md](../explanation/board-size.md)) |
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

## Related pages

- [The hint line](keys-hint-line.md)
- [Add your own keys](../how-to/add-your-own-keys.md)
- [The task panel](task-panel.md)
- [Settings](settings.md)
- [Mouse](mouse.md)
- [The web UI](web-ui.md)
