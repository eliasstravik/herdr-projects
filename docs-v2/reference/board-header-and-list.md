# The board: the header and the list

This page gives the facts about the top part of the board: the header, the five groups of the list, what the list
hides, and how the selection moves. For the rows themselves, see [the rows of the board](board-rows.md).

## The board at a glance

The board is one list of tasks and one chat. The header at the top shows whose tasks show and how many tasks need
you. The list sorts the tasks by what they need from you. The chat at the bottom talks to the Coordinator, or to the
agent of the selected task.

```
 herdr-projects  proj · 3 need you · 1 ready · 2 working · 12 to do · 30 done
 needs you
 ! HSW-98  Add an undo for clear                                       R#2 PR#18 12m ago
           Review failed · 1 critical, 1 major
 ready to approve
 ◆ HSW-99  Max length for task titles                                  R#3 PR#60  2m ago
           Pass · 1 minor
 working
 ◐ HSW-134 Tidy the first-run questions             ███░░░░░ 40% 12m left  R#1 PR#17  5m
           Running the tests
 to do
 ○ HSW-9   Rename the hint line                                                   2h ago
 ○ Coordinator  idle
╭────────────────────────────────────────────────────────────────────────────────────────────╮
│ Coordinator:                                                                               │
│ Added proj/4: Add an undo. An agent is on it.                                              │
│                                                                                            │
│ ❯ Send message to Coordinator                                                              │
╰────────────────────────────────────────────────────────────────────────────────────────────╯
  alt+c hide the latest message · ? all keys
```

| Key | What it does |
| --- | --- |
| `↑` `↓` | Selects a task. |
| `Enter` | Goes into the agent of the selected task. |
| `Space` | Talks to the agent of the selected task, not to the Coordinator. |
| Type, then `Enter` | Sends the message. |
| `Tab` | Sends the message and starts an agent on what the message adds. |
| `Alt+P` | Shows the [task panel](task-panel.md) next to the list. |
| `?` | Lists every key ([keys](keys.md)). |

## The header

The header is one line. It shows where you are: whose tasks show, then how many tasks are in each group. Example:
`herdr-projects proj · 3 need you · 1 ready · 2 working · 12 to do · 30 done`.

- Usually, the header names the repo that new tasks go to. The header shows no folder path.
- If the tasks of other machines show, the header shows that first, in the accent colour. Examples:
  `all machines: otherbox`, `only otherbox` ([other machines](../explanation/other-machines.md)).
- The board never cuts the names of the machines. If the names are too long for the line, each name that does not fit
  goes, whole, to the next line.
- A filter has a line of its own under the header, in the accent colour. That line shows the filter, how many tasks
  match, and the key that clears the filter: `filter "import" · 1 of 33 · esc clears`.
- If no task matches the filter, the list shows `Nothing matches "zzz".`

## The five groups

The tasks are in five groups. The groups are in the order in which they need you.

| Group | It holds |
| --- | --- |
| `needs you` | A question, something that went wrong, a merge that failed, or finished work whose review failed. |
| `ready to approve` | Finished work that passed its review (or had no review). It waits for your yes. |
| `working` | Tasks that an agent or the reviewer works on. |
| `to do` | Tasks that nobody started. |
| `done` | Finished tasks, newest first. |

- The groups that wait for you come first. Then come the tasks that have work in progress (the agent, the reviewer,
  or the check of the board). Then come the to-do tasks. The done tasks come last.
- The done group never pushes the to-do tasks down, however long it grows
  ([why](../explanation/board-design.md#the-order-of-the-groups)).
- A pinned task stays at the top, above all the groups.
- If you group the list by repo (`Alt+S`), the rows of each repo use the same order. Thus the done rows of a repo are
  below the to-do rows of that repo.
- Only the latest five done tasks show. The line under them counts the other done tasks:
  `↓ 73 more done (alt+d shows them)`. `Alt+D` shows all the done tasks.
- The archive (`Alt+A`) is a list of its own, newest first, outside this order.
- The automations are a group of their own, after the task groups. They are in the part of the list for their
  machine. If you group by repo, they are under their repo ([automations in the list](board-automations.md)).

## Moving through the list

- `↑` and `↓` follow the order on the screen.
- `Pgup` and `Pgdn` move ten tasks at a time.
- If you change the grouping, the selected task stays selected.
- The arrows also stop on the headings. `Enter` on a heading folds it to one line. `Enter` again unfolds it.
- If every machine shows, each machine has a heading of its own ([list headings](list-headings.md)).
- The highlight of the selected task covers all its lines. `↑` and `↓` move from task to task, never onto a second
  line of a row.

## The list says what it hides

If the list is longer than its room, the last line of the list shows how many tasks are not on the screen. This count
includes the tasks in folded groups.

- Below the list: `↓ 11 more`.
- Above the list, after you scroll: `↑ 4 more`.
- The list counts the automations that do not show separately, by name: `↓ 3 more, 2 automations`,
  `↓ 7 automations`, `↑ 4 more, 1 automation`.
- A heading never stands alone at the bottom of the screen with nothing under it.
- A list of fewer than eight lines uses every line for tasks, and shows no count.

## A list taller than the window

If the list is taller than the window, rows give way in this order:

1. The done rows give way first. The list shows the latest five done tasks. If the list still does not fit, it shows
   one fewer done task for each line that it is short, down to the newest one. The line under them shows how many more
   there are. The `?` menu has the key that shows all of them.
2. Then the to-do rows give way, from the last one up, down to the first one. A line under them counts the others:
   `… 4 more to do`.

Thus the live tasks stay on the screen. This is also true on a phone, where each live task uses three to five lines.

- The selected task never gives way.
- No row gives way while you search, or after you ask for all the done tasks.
- A to-do task that gave way is still one `↓` away. The arrows go through every to-do task. The to-do tasks that show
  are then the ones that end at the selected task.
- To-do tasks give way only on a list that has live tasks. If there are no live tasks, the list scrolls, and it uses
  no line for a count.
- The list scrolls only if it is still too tall. It scrolls by whole rows: it never shows a part of a row that has more
  than one line. The lines of a row come into view together. If only some lines of a row fit at the bottom, those
  lines stay empty.
- In a list that has only one line, the selected task shows its name.

## The selected row

The selected row has a mark. Each line of the row starts with a bar in the gutter (`▌`), on the tint of the theme.

- The mark is on the selected row of every list: tasks and automations, the archive, the settings page, and the `?`
  menu.
- If the board cannot paint a background (`NO_COLOR`, or a theme that has no colour for it), the row shows in inverse
  video. Thus the selection never depends on colour.
- A click on a row selects it ([the mouse](mouse.md)).

### Where the selection goes after an action

Some actions move the selected task to another group, to another place in the list, or out of the list. These actions
are: put an agent on it, say yes, send it back, stop, pin, and archive. After such an action, the selection does not
follow the task:

- The selection goes to the next task below, in the same group. If the task was the last one in its group, the
  selection goes to the task above.
- If the group is now empty, the selection goes to the first task of the next group. If that group is folded, the
  selection goes to its heading. If there is no next group, the selection goes to the group above.

More rules:

- If a task moves into a folded group, that group stays folded.
- Under a filter, the selection goes to the next task that matches.
- If you group by repo, the same rules apply inside the repo.
- If the task leaves the list (it is archived), the message tells you which task is now selected.
- An action that does not move the task keeps the selection on that task.
- The selection stays on the task while the task panel shows the whole task, while you talk to its agent, and while
  you are in its agent.

## Related pages

- [The rows of the board](board-rows.md)
- [Marks and colours on a row](board-marks-and-colours.md)
- [The board in a narrow window](board-narrow-window.md)
- [Automations in the list](board-automations.md)
- [The chat of the board](board-chat.md)
- [List headings](list-headings.md)
- [Find a task](../how-to/find-a-task.md)
- [Fold list headings](../how-to/fold-list-headings.md)
- [Why the board looks as it does](../explanation/board-design.md)
