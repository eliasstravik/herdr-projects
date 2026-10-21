# List headings

**The task list sits under headings, and `Enter` on a heading folds it.** The headings are a repo's name when the list
is grouped by repo (`Alt+S`), or a status (`needs you`, `working`, `to do`, `done` …) when it is grouped by status.

With every machine shown (`Alt+M`, [other machines](other-machines.md)) there is one more level: **a heading per
machine**, its repos (or status groups) under it, and their tasks under those.

```
 ▾ vm-str ─────────────────────────────────────────────────────────
 ▾ herdr-projects · HSW
 ◐ HSW-151 Rewrite the board in Rust on Ratatui           47%   3h
 ○ HSW-179 Remove the "To: Coordinator" label          1h ago

 ▾ open-crm · CRM
 ○ CRM-4   Explore Open CRM as the next-gen CRM         9h ago

 ▸ vm-ssa  1 task ─────────────────────────────────────────────────
```

## The Coordinator's row on top

**Above every heading is the main Coordinator's row**, wherever a Coordinator can run (and on another machine's board
shown by itself, that machine's). It folds nothing: `Enter` there says so. The list still opens on its first task, and
lands where it did after an archive or a fold: `↑` reaches the top row. The archive has none.

It stands for the [Coordinator](coordinator.md), as a machine's or a repo's heading stands for its coordinator: `Alt+P`,
`Alt+T`, `Ctrl+E`, and `Tab` / `Ctrl+X` for [autopilot](autopilot.md). While a coordinator is on autopilot, its row reads as
a task's: autopilot's mark `◎` in the marks' column (left of the caret, which moves one step in), its timing in the columns on the right and what it does on the line
under it; the top row also sums up the others
(`Coordinator · 2 others on · 1 needs you`).

## Machine and repo headings

- **A machine's heading** is its name in bold, in the accent colour, with a rule to the edge of the list. This machine
  shows its own name (its hostname).
- Machines come in the order of Herdr's machines sidebar ([machine order](machine-order.md)).
- **Every machine that answered has a heading, also one with no task yet.** It shows folded, `▸ vm-ssa  0 tasks`, and
  does what any machine's heading does: `Space` talks to its machine coordinator, which adds tasks there
  ([machine and project coordinators](machine-and-project-coordinators.md)), and its automations show under it. `Enter`
  has nothing to unfold and says so; its first task opens it. While a filter is on, or in the archive, a machine with
  nothing that matches has no heading.
- One whose tasks cannot be read has no heading: it says so above the list, as always.
- **A row never names its machine.** Its tag is the one it has on its own machine's board: the repo's key and number
  (`HSW-151`, [task numbers](task-numbers.md)). The heading above says whose it is.
- **A repo's heading** is its name in bold, in the text colour of the task titles under it, with its key beside it,
  dimmed (`herdr-projects · HSW`), never `vm-str:herdr-projects`. It is never quieter than its tasks, in any theme, so
  a list of several repos reads as several blocks. It has no rule: the rule is a machine's. On each machine, the repo the board is in comes first.
- **On one machine** (this machine's board, or one other machine by itself) there is no machine heading.

## Folding

- **`Enter` on a heading folds it.** Its tasks hide, its caret turns to `▸`, and it says how many tasks it holds
  (`▸ open-crm  4 tasks`), and how many automations where it holds some (`▸ vm-str  7 tasks, 1 automation`,
  [automations](automations.md)). `Enter` again unfolds it.
- **A click on a heading** does the same ([mouse](mouse.md)).
- The hint line says what `Enter` will do (`enter fold this repo`, `enter unfold this machine`,
  `enter fold this group`), and the `?` menu lists it on the line for `enter`.
- **A machine folds whole**: all its repos and tasks hide under its one line, which counts them all. Unfold it and its
  repos are as they were: a folded one is still folded.
- It works the same grouped by repo or by status, on one machine or all of them. The two groupings keep their own
  folds: a repo folded in the repo grouping still shows its tasks in the status grouping.

### How a heading looks

- A heading that folds has a **caret** in the marks' column, in the muted colour: `▾` open, `▸` folded. Its name starts
  one cell after it, where a task's tag starts.
- On a narrow list the caret stays and the heading is cut at the right edge.
- The arrow keys stop on every heading, like on a row. A selected heading has the same bar and tint as a selected row.

### Folds are remembered

They stay while the list refreshes, when you switch the grouping or the machines and back, and **after the board is
closed**. They are kept with the rest of the board's view, in `view.json` in the board's data folder. The popup has its
own, as it has for its selection. The web board keeps its own folds the same way, in its own file.

### What a fold never hides

- **A filter's matches.** While find (`Ctrl+F`) or a kept filter is on, every task that matches shows, folded or not.
  The folds come back when the filter is cleared.
- **The selected task.** When the selection is brought to a task from elsewhere (Herdr's approve action, the popup
  opened from a task's pane), or the list is laid out again (another grouping, other machines), the task's machine and
  group unfold, so it is always in view.

A list longer than its room counts what it does not show on its last line (`↓ 11 more`). Tasks under a folded heading
are not in that count, since the heading already says how many it holds.

## The web board

Same headings, same order: each machine's name over its groups, with a line under it, and each group with its count.

- A tap on a heading (machine or group) folds or unfolds it. Its caret shows which (`▾` open, `▸` folded).
- A folded heading's count is how many tasks it holds.
- A watching device may fold too. It only changes what that page shows.
