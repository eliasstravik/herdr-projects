# Task numbers

This page tells how the board names a task: with the short key of its repo and its number. For how a repo gets its
key and the rules of a key, see [repo keys](task-numbers-repo-keys.md).

## The name of a task

**Every task has the short key of its repo and its number: `HSW-134`.** The board uses that name on every row and in
every message. It never uses a bare `#134`.

The full id of the task, `herdr-projects/134` (the name of the repo and the number), also works in all places, and it
never changes.

- Every repo gets a key when the board first sees it. The board makes the key from the name of the repo
  (`gtm-skills` → `SKL`).
- To change a key, use `herdr-projects set repo.key HSW --repo herdr-projects`
  ([change a repo key](../how-to/task-numbers-change-a-repo-key.md)).
- You can type `HSW-134` (in any letter case) in every place that takes an id: `herdr-projects delegate HSW-134`.

## The tag on a row

```
 ready to approve
 ! HSW-98  Add an undo for clear                                       R#2 PR#18 12m ago
           Fail · 1 critical, 1 major
 working
 ◐ HSW-134 Tidy the first-run questions               ███░░░░░ 40% 12m  R#1 PR#17  5m
           Running the tests
 to do
 ○ HSW-9   Rename the hint line                                                   2h ago
```

- **Where:** to the right of the status mark, to the left of the title: `HSW-134`, the key of the repo and the number.
- **Colour:** the theme's grey for greyed-out words. It is the same on every row and in every state, so that you read
  the title first.
- **One column.** All tags share one column, as wide as the widest tag of the list. The widest tag includes all tasks
  of the list, also folded tasks, so the column does not move when the list folds. The board adds spaces after a
  shorter tag. Thus every title starts in the same place, also when ids have different lengths: `HP-1` lines up with
  `HSW-206`. Tags start at the left:

  ```
   ○ HP-1      Tidy the first-run questions
   ○ CRM-2     Rename the hint line
   ○ HSW-206   Add an undo for clear
   ○ HSWXY-140 A task with a long key and a long number
  ```

  The second line of a live task (what it says) starts under its own title, not under the tag.
- **The tags on the right are different.** `R#2` is the review round and `PR#18` is the pull request. They are at the
  right edge, in the colour of the result of the review and the checks. The task tag is at the left, in grey, and has
  no `#`: a dash joins the key to the number.
- **Several repos in one list:** the key tells them apart. In every list, a tag is the key of the repo with the number
  (`PW-12`). Thus two tasks with the number 1 in two repos show as `PW-1` and `HSW-1`.
  - To see the tasks of each repo together, group the list by repo: press `Alt+S` (in the `?` menu, "group by repo:
    see which repo each task is in"; on the web board, the "Group by repo" button). The heading of each group names
    its repo.
  - The whole task also names the repo, in its `Task id` row (`pi-work/12`), with the session.
- **Other machines.** A task of another machine has the tag that its own machine gives it (with the keys of that
  machine). Each machine counts its own tasks, so two machines can each have a `#1`.
  - When the board shows every machine, each machine is a heading with its tasks under it
    ([list headings](list-headings.md)). Thus a row has the plain tag (`HSW-1`), never the name of its machine.
  - Where there is no heading above (the messages of the board, the chat, the whole task), the name is
    `otherbox:HSW-1`.
  - Find takes the tag alone, without the machine.
  - A machine can still have an older version, where a repo has no key. A task of that repo shows the number that
    its machine gives: `#1`, `otherbox#1`.
- **When the space is too small**, the tag is the last item to go before the rest of a name. These items go first: what
  a row says, the bar, the word `ago`, the end of a long name, the percent with the minutes left, the `R#` tag with the
  pull request, and the time. [The narrow window](board-narrow-window.md) has the full order.
  - The tag goes for the whole list at the same time, so that the titles stay in one column. This occurs when the row
    with the widest tag has fewer than 16 cells for its name with nothing else next to it (a list under about 25
    columns).
  - The column takes a few cells from every row. Thus each of the other items goes that many columns earlier than
    before the tag column.
- **On a narrow screen** (under 80 columns), the name of a live row wraps. The lines of the name, of what the task
  says, and of the bar all start under the title. Thus a row has one left edge.

## All other places that name a task

| Where | What it shows |
|---|---|
| the list, the archive, find: rows of one, two and three lines, in every group | the tag, as above |
| the task panel and the details page | the tag before the title, greyed. The heading is `Task details  HSW-134 "its title"`. The id (`herdr-projects/134`) shows once, on the whole task, as `Task id` with the session, where a command needs it. |
| the line of the chat that names who you talk to | the tag between the mark and the name of the task. If the window is too narrow to leave 20 cells for the status, the tag does not show. |
| the firings of an automation | `made HSW-140`, on the board and on the command line |
| the messages of the board | the tag and the title, in one form: `Archived HSW-134 "Fix the footer".`, `Pinned HSW-14 "…" to the top.`, `Stopped HSW-14 "…".`, `Removed …`. A message that an action wrote with the id also shows the tag (`HSW-14: approved, merging`). Only a line that shows a command to run keeps the id that the command takes. Pinning says so. An action on another machine says what it does there (`Pinning HSW-2 on otherbox…`), never a bare `…`. |
| Herdr's sidebar, and the title of a pane | The name of the task's space is the tag and the title of the task (`HSW-34 Add a file…`). That is the first row of its agent's entry ([Herdr's sidebar](sidebar.md)). The title of the pane is the same, so the sidebar of the web UI also shows `HSW-34 Add a file…`. A reviewer's pane has a name of the same form. |
| the web board | the same tag on each row, in the same column, with the same limit on the space before a title. What a task says and its bar start under the title, not under the tag. If the list is too narrow to leave 16 letters for the title of the widest tag, no row shows a tag. The tag is before the title of an open task, and in the line of the chat. |

### The key next to the name of a repo

Where the board names a repo, the key is next to the name, dimmed: `gtm-agent · AGT`. These places are:

- the top line of the board, which names the repo that new tasks go to
  (`herdr-projects  gtm-agent · AGT · 3 to do`), also on the web board;
- the heading of a repo in the list grouped by repo, also on the web board;
- the `Task id` row of the whole task (`gtm-agent/12 · AGT`), and the `Repo` row of an automation.

The key is never next to the name on the row of a task, because the row already starts with the key (`AGT-12`). On a
narrow screen, the board cuts a heading at its end. Thus the key goes first and the name stays.

### Short form or id

Every place that names a task in words uses the short form:

| Where | With the key `HSW` |
|---|---|
| the answer and the error of a command (`describe`, `delegate`, `archive`, …) | `HSW-134: updated (description)` |
| the messages of the board, also what a command answered through the board | `HSW-135: restored` |
| what the agent of a task is told about its own task | `its id on the work board is herdr-projects/134; the human also calls it HSW-134, and every command takes that name too` |
| the body of a pull request, and the commit that the board makes | `Task: HSW-134 (herdr-projects/134)` |

The board keeps the id where a program reads it, because the id is the real name of the task and it never changes:

- `--json`;
- branch names;
- folders and paths (the board never rewrites an id in a path);
- the commands that an agent gets to run.

The first column of `herdr-projects list` is the short name, lined up.

## The key names the task

You can use `HSW-134`, in any letter case, in every place that takes an id. This is true for every command:
`details`, `delegate`, `tell`, `stop`, `approve`, `review`, `retry`, `fix`, `finish`, `report`, `describe`, `pin`,
`archive`, `restore`, `clear`, `show`, `set … --task`. Thus "start HSW-134" is `herdr-projects delegate HSW-134`. The
rules of the Coordinator and the skill also say this.

The id continues to work and stays the real id of the task: the board stores nothing under a key. If a key changes,
nothing breaks:

- The id still names the task.
- The old key also still names the task (`HS-134` after `HS` became `HSW`).
- This is true for every type of change: by `set`, by hand in your settings, or in the file of the repo.

The board records every key that a repo uses, in `~/.herdr-projects/data/repo-keys.json`, as a key of that repo
forever. No other repo can take it, by `set` or by its file.

To find a task by its number or tag, see [find a task](../how-to/find-a-task.md).

## Related pages

- [Repo keys](task-numbers-repo-keys.md)
- [Change a repo key](../how-to/task-numbers-change-a-repo-key.md)
- [Find a task](../how-to/find-a-task.md)
- [List headings](list-headings.md)
- [Herdr's sidebar](sidebar.md)
- [The narrow window](board-narrow-window.md)
- [Commands](commands.md)
