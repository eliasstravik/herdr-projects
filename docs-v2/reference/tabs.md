# A task's tabs in Herdr

This page lists the tabs that the board opens in Herdr for a task, and when they open and close.

## The tabs

Every task with an agent has its own Herdr space. In that space, each helper has its own tab:

| Tab | Who is in it |
|---|---|
| `Agent` | the agent that does the task |
| `Reviewer #1`, `Reviewer #2`, … | the reviewer, one tab for each review round |
| `Planner` | the agent that writes the plan of the task ([a plan first](../explanation/plan.md)) |
| `Plan review` | the reviewer that checks that plan |
| `Agent A`, `Agent B`, … and `Picker` | [several attempts](../explanation/attempts.md) at one task, and the reviewer that compares them |

When a [fallback agent](../explanation/fallback.md) takes over, the tab shows the program that is on it now:
`Agent (codex)`, `Reviewer #2 (claude)`.

Two tabs are outside the task spaces:

- `Coordinator`, in the home space ([the Coordinator](../explanation/coordinator.md));
- `herdr-projects`, the board itself ([the board tab and popup](board-tab-and-popup.md)).

## When tabs open and close

- **`Agent`** gets its name each time the agent starts: the first time, after a restart, and when the board opens a
  closed space again.
- **`Reviewer #N`** opens when a review starts. It closes when the review ends, whatever the result. It also closes
  when you stop the task, and when a review is cut off and tried again.
  - `N` is the review round. It is the same number as the `R#2` tag of the row.
  - A reviewer is never a pane in the tab of the agent.
- **`Planner`** (set by `plan.agent`) closes after you approve the plan.
- **`Plan review`** is used when `plan.approve` is `reviewer`.
- **Attempts:** when one attempt is picked, the tabs of the other attempts close. The kept tab keeps its name.
- **Done:** when the task is merged or archived, its whole space closes, with all its tabs.

## Nothing jumps

A new reviewer tab does not take your focus. The board tells Herdr not to go to the new tab, so you stay where you
are. The reviewer has its own tab, so the pane of the agent and the board keep their size.

To watch the reviewer, go to its tab. In the meantime, the task panel and the chat show what the reviewer says.

## Details

- The board finds each helper by its pane, not by the name of its tab. If you rename a tab, nothing breaks.
- The board saves the tab and the pane of a running reviewer with the task. If the board restarts during a review, it
  still shows what the reviewer says, takes its verdict and closes its tab.
- Before the next review starts, the board closes a tab that is left from an interrupted review.
- An older version of the board started a reviewer as a pane in the tab of the agent. The board closes that pane and
  does nothing else.

## For contributors

The tab names and their code are in [`crates/flow/src/flow/tabs.rs`](../../crates/flow/src/flow/tabs.rs). What Herdr
does with tabs is in [Herdr 0.9.3 facts](../contributing/herdr-0.9.3-facts.md).

## Related pages

- [Herdr's sidebar](sidebar.md)
- [A plan first](../explanation/plan.md)
- [Several attempts](../explanation/attempts.md)
- [Fallback agents](../explanation/fallback.md)
- [The Coordinator](../explanation/coordinator.md)
- [Herdr 0.9.3 facts](../contributing/herdr-0.9.3-facts.md)
