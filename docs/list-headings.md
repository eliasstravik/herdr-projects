# List headings

**The task list sits under headings, and `←` on a heading folds it, `→` unfolds it.** The headings are a repo's name when the list
is grouped by repo (`Alt+S`), or a status (`needs you`, `working`, `to do`, `done` …) when it is grouped by status.
Grouped by machine (`Alt+S` again), each machine has its heading and its tasks stand right under it, with no status
headings.

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

## The top row

**On top of every list is the top row**: `All machines` with every machine shown, or the one machine shown (on a board
with only this machine, that one). Everything else stands under it, one step in. It says what it holds, open or folded
(`2 machines · 5 tasks`, `9 projects · 30 tasks`), and folds the whole list away. It is its coordinator's row, as every
machine's and project's row is ([machine and project coordinators](machine-and-project-coordinators.md)), and the board
opens on it. The archive has none.

## Machine and repo headings

- **A machine's heading** is its name in bold, in the base colour, nothing after it (no rule); its coordinator's mark, where it has one, is left of its caret. This machine
  shows its own name (its hostname).
- Machines come in the order of Herdr's machines sidebar ([machine order](machine-order.md)).
- **Every machine that answered has a heading, also one with no task yet.** It shows folded, `▸ vm-ssa  0 tasks`, and
  does what any machine's row does: `Space` talks to its machine coordinator, which adds tasks there
  ([machine and project coordinators](machine-and-project-coordinators.md)), and its automations show under it. `→`
  has nothing to unfold and says so; its first task opens it. While a filter is on, or in the archive, a machine with
  nothing that matches has no heading.
- One whose tasks cannot be read has no heading: it says so above the list, as always.
- **A row never names its machine.** Its tag is the one it has on its own machine's board: the repo's key and number
  (`HSW-151`, [task numbers](task-numbers.md)). The heading above says whose it is.
- **A repo's heading** is its name in normal weight, in the base colour of the task titles under it, one step in, with its key beside it,
  dimmed (`herdr-projects · HSW`), never `vm-str:herdr-projects`. It is never quieter than its tasks, in any theme, so
  a list of several repos reads as several blocks. It has no rule: the rule is a machine's. On each machine, the repo the board is in comes first.
- **On one machine** (this machine's board, or one other machine by itself) the machine's row is the top row.

## Folding

- **`←` on a heading folds it.** Its tasks hide, its caret turns to `▸`, and it says how many tasks it holds
  (`▸ open-crm  4 tasks`), and how many automations where it holds some (`▸ vm-str  7 tasks, 1 automation`,
  [automations](automations.md)). `→` unfolds it. `←` on a task's row goes to its heading.
- **`Enter` on a heading folds it and unfolds it too**; on a task's row it does nothing. `Alt+O` goes into a row's
  agent: on a machine's or a project's row its coordinator, on a task its agent.
- **A click on a heading** folds it and unfolds it ([mouse](mouse.md)).
- The hint line says what `Enter` will do (`enter fold`, `enter unfold`), and the `?` menu lists `Enter`, `←` and `→`.
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

Same rows, same order, same depth: the top row at the left edge, each machine's row one step in under `All machines`,
a project's and a status group's one step in under its machine, a project's tasks one step further. Every name is in
the base colour (the top row's and a machine's bold), a status group's heading in its colour, what goes with a name
dimmed.

- A machine's row and a project's row are their coordinator's: its mark where a task's mark stands, its time on the
  right, one line. A tap selects it (its coordinator is then the other one the chat's switch offers); its caret (`▾`
  open, `▸` folded) folds and unfolds it. Selected, its **Open** shows the coordinator whole.
- A status group's heading, the automations' and the archive's have nobody to talk to: a tap anywhere on them folds and
  unfolds them, their caret standing in the marks' column.
- A folded heading says what it holds (`7 tasks, 1 automation`, `0 tasks`); the top row says it open or folded
  (`2 projects · 14 tasks`).
- A watching device may fold too. It only changes what that page shows.
