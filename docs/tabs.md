# A task's tabs in Herdr

Every task with an agent gets its own Herdr space. Inside it, each helper has its own tab:

| Tab | Who is in it |
|---|---|
| `Agent` | the agent doing the task |
| `Reviewer #1`, `Reviewer #2`, … | the reviewer, one tab per review round |
| `Planner` | the agent writing the task's plan ([a plan first](plan.md)) |
| `Plan review` | the reviewer checking that plan |
| `Agent A`, `Agent B`, … and `Picker` | [several attempts](attempts.md) at one task, and the reviewer who compares them |

When a [fallback agent](fallback.md) takes over, the tab says which program is on it now: `Agent (codex)`,
`Reviewer #2 (claude)`.

Two tabs live outside the task spaces: `Coordinator` in the home space ([the Coordinator](coordinator.md)) and
`herdr-projects`, the board itself ([the board](board.md)).

## When tabs open and close

- **`Agent`** gets its name each time the agent starts: the first time, after a restart, and when the board reopens a
  space that was closed.
- **`Reviewer #N`** opens when a review starts and closes when it ends, whatever the result. It also closes when you
  stop the task, and when a review is cut off and tried again. `N` is the review round, the same number as the row's
  `R#2` tag. A reviewer is never a pane inside the agent's tab.
- **`Planner`** (set by `plan.agent`) closes after you say yes to the plan. `Plan review` is used when
  `plan.approve` is `reviewer`.
- **Attempts:** when one attempt is picked, the other attempts' tabs close. The kept one keeps its name.
- **Done:** when the task is merged or archived, its whole space closes, tabs and all.

## Nothing jumps

A new reviewer tab does not take your focus. Herdr is told not to switch to it, so you stay where you are. Because it
is its own tab, the agent's pane and the board keep their size. Go to the tab to watch the reviewer. Meanwhile the task
panel and the chat show what it says.

## Details

- The board finds each helper by its pane, not by its tab's name. Renaming a tab yourself breaks nothing.
- A running reviewer's tab and pane are saved with the task. If the board restarts mid-review, it still shows what the
  reviewer says, takes its verdict and closes its tab.
- A tab left over from an interrupted review is closed before the next review starts.
- A reviewer started by an older version of the board, as a pane in the agent's tab, has that pane closed and nothing
  else.

The names and the code are in `crates/flow/src/flow/tabs.rs`. What Herdr does with tabs is in
[checked on Herdr](checked-on-herdr.md).
