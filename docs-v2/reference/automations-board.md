# Automations on the board

This page lists how the board and the web board show automations, and the keys that act on them. For the commands and
the triggers, see [automations](automations.md).

## Where automations show in the list

An automation belongs to its machine, as a task does. The list shows it there
([board rows](board-rows.md)). There is no separate list behind a key now: `Alt+O` is not a key of the board.

- **By status:** the automations of a machine are a group of their own under the heading `automations`. This group
  comes after the task groups of the machine (`working`, `to do`, `done` …). With every machine shown (`Alt+M`), the
  automations of each machine are in the part of the list of that machine, under its heading. On the board of one
  machine, they are the automations of that machine. A machine with no automation has no such heading.
- **By repo** (`Alt+S`): each automation is under the heading of its repo, after the tasks of that repo. It has no
  heading of its own.
- **Folding** ([list headings](list-headings.md)): `Enter` on the `automations` heading folds the group to one line
  (`▸ automations  2 automations`). A folded machine or repo hides its automations with its tasks and counts both:
  `▸ vm-str  7 tasks, 1 automation`. If something takes you to an automation in a folded group (the link of a firing,
  a jump), its group unfolds.

```
 ▾ vm-str ──────────────────────────────────────────────────────────────────────────────────
 ▾ to do
 ○ HSW-2 Write the changelog                                                          1d ago

 ▾ automations
 ⟳ on    Daily triage of outside issues and PRs  daily 11:00                    next in 19h

 ▸ otherbox  3 tasks, 2 automations ────────────────────────────────────────────────────────
```

## The row of an automation

There is one row for each automation, in the places of a task row ([board rows](board-rows.md) has the table):

- its mark, `⟳`, dim while it is off;
- the word `on` or `off` where a task has its number (never colour alone);
- its name where the titles start, and its trigger in plain words after it (`every 2h`,
  `when a task is merged, in every repo`);
- its repo, if that is not the repo that the board shows (when grouped by repo, the heading gives it). Never its
  machine: the heading above gives that;
- the task that its last firing made, by the mark and the number of that task (`✓ HSW-4`);
- in the time column of the list: `next in 4m` for a schedule that is on. Else `fired 2h ago` (`skipped 2h ago` if
  the last firing made no task). Else `never fired`.

If an automation turned itself off, a line under its row tells why. An automation that is off is drawn as quietly as
a done task. If the window is too narrow for the trigger beside the name, the trigger goes on a line under it.

**Colour:** the heading, the marks and the `[auto]` tag of a task that an automation made use a purple that no status
has ([colours](colours.md)).

## Keys on the row

Select the row with `↑` `↓` where it is, or with a click.

| Key | Does |
|---|---|
| `Enter` | open it |
| `Space` | turn it on or off |
| `Tab` | fire it now |
| `Ctrl+X` | remove it (it asks first, as removing a task does; the tasks it made stay) |
| `Ctrl+E` | its settings |

- The hint line shows these keys while such a row is selected. It shows only the keys that act. An automation that is
  off never fires, so the hint line does not show `Tab` for it.
- The `?` menu lists them in the same words.
- A key that acts on a task (approve, send back, archive, pin) does nothing on an automation.

## One automation, whole

Open it with `Enter` or `Alt+T`. On a wide screen, `Alt+P` shows it beside the list. It shows:

- its state, its trigger, when it is next due, and its repo;
- its prompt;
- its own settings, marked `*`;
- its firings, newest first, each with the task that it made or why it skipped.

`↑` `↓` pick a firing. `Enter` on a firing goes to that task in the list (in the archive, if the task was archived
since). `Esc` goes back to the list.

## Its settings page

`Ctrl+E` opens the same page that a task has ([the task panel](task-panel.md)). It shows each work setting as the next
task will have it. The rows that are the own values of the automation have `*`. To see where the other values come
from, use `show --automation`. `Enter` changes a row and `Ctrl+U` puts it back.

## A task that an automation made

Its row shows `[auto]` after its name, in the colour of automations. The Status of its panel shows `Made by` with the
name of the automation.

## Automations of other machines

They show with the tasks of that machine, in the part of the list of that machine
([other machines](../explanation/other-machines.md)). The board of one machine shows only its own automations.

- The keys are the same in the part of any machine. The machine that has the automation turns it on and off, fires it
  and removes it.
- The whole automation comes from what that machine sent (its latest twenty firings).
- `Ctrl+E` opens its settings as for this machine. The rows are the own rows of that machine (each work setting as its
  next task will have it there). A row that you change or put back changes on that machine.
- If a machine has a version that sends no such rows, there is no page. The board shows the command in its place.

## The web board

The [web board](web-ui.md) has the same groups in the same places. A folded heading counts the same way:
`1 task, 1 automation`. It has the same whole automation and the same actions, drawn from the same code:

- a row that you tap to open;
- buttons to turn it on or off, fire it now and remove it;
- each firing that made a task, as a link to that task.

A row shows all of its text, however narrow the page is (a task row keeps to two lines). On or off, and when it fires,
are never cut. The settings page of an automation is only on the terminal board. On the web, change the settings with
the command. The popup is the same board, so it has all of this too.

## Related pages

- [Automations](automations.md)
- [Make an automation](../how-to/make-an-automation.md)
- [Board rows](board-rows.md)
- [List headings](list-headings.md)
- [Colours](colours.md)
- [The task panel](task-panel.md)
- [Keys](keys.md)
- [The web UI](web-ui.md)
