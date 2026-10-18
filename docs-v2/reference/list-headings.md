# List headings

This page tells how the headings of the task list look and how they fold. To fold or unfold a heading, see
[fold list headings](../how-to/fold-list-headings.md).

The task list is under headings. The headings are:

- the name of a repo, when the list is grouped by repo (`Alt+S`);
- a status (`needs you`, `working`, `to do`, `done` …), when the list is grouped by status.

When the board shows every machine (`Alt+M`, [other machines](../explanation/other-machines.md)), there is one more
level: **a heading for each machine**. Its repos (or status groups) are under it, and their tasks are under those.

```
 ▾ vm-str ─────────────────────────────────────────────────────────
 ▾ herdr-projects · HSW
 ◐ HSW-151 Rewrite the board in Rust on Ratatui           47%   3h
 ○ HSW-179 Remove the "To: Coordinator" label          1h ago

 ▾ open-crm · CRM
 ○ CRM-4   Explore Open CRM as the next-gen CRM         9h ago

 ▸ vm-ssa  1 task ─────────────────────────────────────────────────
```

## Machine headings

- **A machine heading** is the name of the machine in bold, in the accent colour, with a line to the edge of the list.
  This machine shows its own name (its hostname).
- The machines are in the order of Herdr's machines sidebar ([machine order](../explanation/machine-order.md)).
- **Every machine that answered has a heading, also a machine with no task.** Such a heading shows folded:
  `▸ vm-ssa  0 tasks`. It does what every machine heading does:
  - `Space` talks to its machine coordinator, which adds tasks there
    ([machine and project coordinators](../explanation/machine-and-project-coordinators.md)).
  - Its automations show under it.
  - `Enter` has nothing to unfold, and the board says so. The first task of the machine opens the heading.
- While a filter is on, or in the archive, a machine with no matching task has no heading.
- A machine whose tasks the board cannot read has no heading. The board says so above the list.
- **A row never names its machine.** Its tag is the tag that the task has on the board of its own machine: the key of
  the repo and the number (`HSW-151`, [task numbers](task-numbers.md)). The heading above the row shows the machine.
- **On one machine** (the board of this machine, or one other machine alone), there is no machine heading.

## Repo headings

- **A repo heading** is the name of the repo, with its key next to it, dimmed: `herdr-projects · HSW`. It is never
  `vm-str:herdr-projects`.
- On each machine, the repo that the board is in is first.

## Folding

- **`Enter` on a heading folds it.** Its tasks hide, its caret changes to `▸`, and it shows how many tasks it holds
  (`▸ open-crm  4 tasks`). If it holds automations, it also shows how many (`▸ vm-str  7 tasks, 1 automation`,
  [automations](automations.md)).
- `Enter` on a folded heading unfolds it.
- **A click on a heading** does the same ([the mouse](mouse.md)).
- The hint line shows what `Enter` will do (`enter fold this repo`, `enter unfold this machine`,
  `enter fold this group`). The `?` menu shows it on the line for `enter`.
- **A machine folds as a whole.** All its repos and tasks hide under its one line, and the line counts all of them.
  When you unfold the machine, its repos are as they were before: a folded repo stays folded.
- Folding works the same when grouped by repo or by status, on one machine or on all machines.
- Each grouping keeps its own folds. A repo that is folded in the repo grouping still shows its tasks in the status
  grouping.

### How a heading looks

- A heading that folds has a **caret** in the column of the marks, in the muted colour: `▾` is open, `▸` is folded.
- The name of the heading starts one cell after the caret, where the tag of a task starts.
- On a narrow list, the caret stays and the heading is cut at the right edge.
- The arrow keys stop on every heading, as on a row.
- A selected heading has the same bar and tint as a selected row.

### The board keeps the folds

The folds stay:

- while the list refreshes;
- when you change the grouping or the machines, and change them back;
- **after you close the board**.

The board keeps them with the rest of its view, in `view.json` in the data folder of the board. The popup has its own
folds, as it has its own selection. The web board keeps its own folds in the same way, in its own file.

### What a fold never hides

- **The matches of a filter.** While find (`Ctrl+F`) or a kept filter is on, every matching task shows, folded or not.
  When you clear the filter, the folds come back.
- **The selected task.** The task's machine and group unfold when the selection moves to a task from a different place
  (Herdr's approve action, or the popup opened from the pane of a task). They also unfold when the list is laid out
  again (a different grouping, other machines). Thus the selected task is always in view.

A list that is longer than its space counts the rows that it does not show, on its last line (`↓ 11 more`). This count
does not include the tasks under a folded heading, because the heading already shows how many it holds.

## The web board

The web board has the same headings in the same order. Each machine name is above its groups, with a line under it.
Each group shows its count.

- A tap on a heading (machine or group) folds or unfolds it. Its caret shows which: `▾` is open, `▸` is folded.
- The count of a folded heading is the number of tasks it holds.
- A watching device can also fold headings. This changes only what that page shows.

## Related pages

- [Fold list headings](../how-to/fold-list-headings.md)
- [Task numbers and repo keys](task-numbers.md)
- [The mouse](mouse.md)
- [Other machines](../explanation/other-machines.md)
- [Machine order](../explanation/machine-order.md)
- [Machine and project coordinators](../explanation/machine-and-project-coordinators.md)
- [Automations](automations.md)
