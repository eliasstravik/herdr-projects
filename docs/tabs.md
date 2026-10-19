# A task's tiled panes in Herdr

Every task with an agent gets its own Herdr space. The first agent is started in a split pane beside the task's anchor
terminal. Reviewers and planners also open as split panes, never as new tabs:

| Pane | Who is in it |
|---|---|
| Agent pane | the agent doing the task |
| Reviewer pane | the reviewer for each review round |
| Planner pane | the agent writing the task's plan ([a plan first](plan.md)) |
| Plan review pane | the reviewer checking that plan |
| Attempt panes and picker pane | [several attempts](attempts.md) at one task, and the reviewer who compares them |

When a [fallback agent](fallback.md) takes over, the board shows which program is on the pane.

The `Coordinator` and the `herdr-projects` board remain their own Mission Control surfaces ([the Coordinator](coordinator.md),
[the board](board.md)).

## When panes open and close

- **Agent panes** are created as splits when an agent first starts; restarts reuse the pane.
- **Reviewer panes** open when a review starts and close when it ends, whatever the result. They also close when you
  stop the task, and when a review is cut off and tried again.
- **Planner panes** (set by `plan.agent`) close after you say yes to the plan. `Plan review` is used when
  `plan.approve` is `reviewer`.
- **Attempts:** when one attempt is picked, the other attempts' panes close.
- **Done:** when the task is merged or archived, its whole space closes.

## Nothing jumps

A new helper pane does not take your focus. The board remains available as Mission Control, while Herdr tiles each
interactive CLI terminal into the task's workspace.

## Details

- The board finds each helper by its pane. The pane and workspace links are saved with the task.
- A running reviewer's pane is saved with the task so the board can resume tracking after a restart.

The split and task-pane code is in `crates/flow/src/flow/tabs.rs` and `crates/flow/src/delegate/launch.rs`. What Herdr does with tabs is in
[checked on Herdr](checked-on-herdr.md).
