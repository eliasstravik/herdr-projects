# Automations in the list

This page gives the facts about the rows of automations in the board list: where they are, what each part of a row
shows, and what gives way when the room is short.

## Where the automations are

- The automations are a group of their own, under the heading `automations`, after the task groups.
- Each machine has its automations in its own part of the list, as it has its tasks.
- The group folds as every group does.
- If you group by repo, each automation is under the heading of its repo, after the tasks of that repo
  ([automations](automations.md)).
- If there are no automations, there is no heading.
- The heading and the mark of each row are in a colour of their own: a purple that no status has
  ([colours](colours.md)).
- There is one row for each automation. The row uses the columns of the list. The code that draws the row of a task
  also draws it.

```
 ▾ to do
 ○ HSW-2 Write the changelog                                                                  1d ago
 ○ HSW-1 Fix the footer on the pricing page                                                   3h ago

 ▾ automations
 ⟳ on    Look for old dependencies  every 2h                                 ✓ HSW-4 next in  4m
 ⟳ on    Release note after a merge  when a task is merged, in every repo            skipped  5h ago
 ⟳ off   Nightly audit  daily 03:00                                                      never fired
         it fired 30 times in an hour
```

## The parts of a row

| Where the row of a task has | The row of an automation has |
| --- | --- |
| Its mark | `⟳` (U+27F3). It is in the colour of the automations while the automation is on, and in the same colour, dimmed, while it is off. It is an arrow that goes round. The mark of every task is round, a diamond or a sign. |
| Its tag (`HSW-12`) | The word `on` or `off`, in the same grey. It fills the column of the numbers, so every name starts where the titles start. |
| Its title | Its name, plain as the title of a to-do task. The name is greyed while the automation is off. |
| What it says, after the title | Its trigger in plain words (`every 2h`, `when a task is merged, in every repo`), in the same grey. Then its repo, if that repo is not the one that the board shows and not the heading above it. It never shows its machine: the heading above names the machine. |
| Its review tag and pull request | The task that its last firing made, with the mark and the number of that task (`✓ HSW-4`: done; `◐ HSW-7`: an agent is on it). Then the word for the time: `next in`, `fired` or `skipped`. |
| Its time | How long until it fires (`4m`), or how long ago it last fired (`2h ago`). It is in the time column of the list, under the times of the tasks. An automation that never fired shows `never fired` there. |
| A second line, for a live task | A second line only if it has something to say: why the board turned it off. If the trigger has no room next to the names, the trigger is also there, on every row of the group at the same time. |

## What gives way when the room is short

The same code as for a task row decides what gives way
([the order in which parts give way](board-narrow-window.md#the-order-in-which-parts-give-way)). The order is:

1. What the row says. It moves to the line under the name. The board never cuts it.
2. The part of a long name after 40 cells.
3. The last task.
4. The firing.
5. The column of the numbers.
6. The rest of the name.

More rules:

- If the column of the numbers goes, the word `on` or `off` goes with it. Thus every name still starts where the titles
  start. The word then becomes the first word of what the row says (`on · every 2h`). On or off is a word at every
  width, never only a colour.
- Under 80 columns, a row is its name, with the trigger under it. The firing is at the end of that line, as a live task
  row has its time there.
- Under 80 columns, a long name wraps on the selected row. On the other rows, the board cuts it with `…`.
- The selected row has the same bar and tint as a selected task. A click selects it.
- `↓ 3 automations` shows how many automations do not fit.

## What is not on the row

These are not on the row: the prompt, the settings, the folder, the list of firings and why one was skipped, the hour
when a schedule is due next, and how many times it fired. The board keeps only the last 200 firings, and another machine
sends 20, so a count would not be correct. To see all of this, open the automation (`Enter`).

## Keys and other rules

- `↑` and `↓` move from the last task to the heading of the automations, then to the automations.
- On the row of an automation, no task is selected. Thus the keys that act on a task do nothing.
- The row of an automation has keys of its own: open it, turn it on or off, fire it now, remove it, and its settings.
  The hint line shows them ([automations](automations.md)).
- An automation that is off is drawn as quietly as a done task: its mark is dimmed and its words are greyed.
- Find (`Ctrl+F`) also matches automations, by name, by trigger or by repo.
- The archive has no automations.
- No task gives way for the automations. If the list is longer than its room, the tasks give way as usual, and the
  automations scroll with the rest.

## Related pages

- [Automations](automations.md)
- [Make an automation](../how-to/make-an-automation.md)
- [How automations work](../explanation/automations.md)
- [The header and the list](board-header-and-list.md)
- [The board in a narrow window](board-narrow-window.md)
- [Why the board looks as it does](../explanation/board-design.md)
