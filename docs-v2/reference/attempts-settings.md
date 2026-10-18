# Attempts settings

This page gives the facts about several attempts on one task: the two settings, the names, what the board shows and the
keys.

## The two settings

| Setting | Values | Built in | Means |
|---|---|---|---|
| `attempts.by` | a count (`3`), or a list of agents (`claude, codex, pi`; a model after a slash; a name can repeat) | `1` | how many attempts are made at the same time, and by whom. A count means that many of `worker.agent` |
| `attempts.pick` | `you`, `reviewer` | `you` | who chooses among the attempts that passed the check |

- **At most 5.** Every attempt is a whole agent with its own worktree. It takes one of the slots of `board.agents`,
  and costs what the task costs when it is done one time. The board refuses a count above 5, and the refusal names 5.
- They are settings the same as all others ([settings](settings.md)):
  - The board looks them up in this order: the task, your entry for the repo, you, built in.
  - You set them with `set` (with `--task`, `--repo` or neither), and remove them with `unset`.
  - You can give them to `add` and `delegate`.
  - An [automation](automations.md) can hold them (`--automation`).
  - `show` lists them, and so does the task's settings page.
- **A repo's own file cannot set them.** Thus code that an agent writes cannot turn on five attempts.
- The board reads `attempts.by` when the task starts, and `attempts.pick` when its attempts are through.
- With a [plan](plan-settings.md), the board reads `attempts.by` when the build starts.
- A task in no repo is always done one time.

## Names of attempts

| Thing | Name |
|---|---|
| An attempt | A letter: `A`, `B`, `C`, … |
| Its branch | `work/12a-…`, `work/12b-…` |
| Its tab in the task's Herdr space | `Agent A`, `Agent B` ([a task's tabs](tabs.md)) |
| The tab where the reviewer compares the attempts | `Picker` |

## When an attempt is out

An attempt is out if one of these occurs:

- its check still fails after `review.rounds` times back to its agent;
- it stopped without a summary;
- it was stopped;
- its agent is gone.

## The pick

| Attempts that passed | `attempts.pick` | Result |
|---|---|---|
| None | either | The task failed (`No attempt passed`). Every attempt's worktree, branch and tab stay |
| One | either | It wins, and nobody is asked |
| Several | `you` | The task waits in the state **pick one**. If there is a reviewer (`review.when` is not `never`), it first compares the attempts in the `Picker` tab and recommends one. You choose |
| Several | `reviewer` | The reviewer compares them, and the one it names continues by itself. If it names none (it failed or timed out), or there is no reviewer, the task waits for your pick and says why |

With `merge.by itself` and `attempts.pick you`, the task stops for your choice, then merges by itself. With
`attempts.pick reviewer`, the task runs to the end and nobody is asked.

## What the board shows

| Where | What it shows |
|---|---|
| The row | One row. A tag after the task's name gives the number of attempts (`×3`). The line under it has a mark for each attempt: `A claude ✓ · B codex working 60% · C pi ✗ check`. The bar is the average of the attempts |
| The row, when an attempt waits for a slot | `C claude waits for a slot` |
| The row, pick one | The task is under **needs you** and says `Pick one of 3`, with the reviewer's recommendation when there is one |
| Herdr's sidebar | The task's space says the same as the row. Each attempt that passed says `pick one` |
| The task panel, part **Attempts** | Headed **Pick one of 3** while it waits. The reviewer's comparison and recommendation on top. Then each attempt: its agent, where it stands, its time, its tokens, the size of its change, what the reviewer said of it, and a link to its diff (for a research-only task, to its report) |
| The task panel, marks | `›` marks the selected attempt. `★` marks the reviewer's choice, or the attempt that was kept |
| The web board | The same row and part. The keys are buttons: `Next attempt`, `Yes` |
| `herdr-projects show <id>` | Each attempt's agent, and the one chosen |
| `details <id>` | The attempts and the comparison |

The [Watcher](../explanation/watcher.md) follows each attempt's agent.

## Keys and commands

| Key or command | What it does |
|---|---|
| `Ctrl+N` | Moves to the next attempt |
| `Ctrl+L` | The approve key: keeps the selected attempt. It asks first |
| `Enter` | Goes into the selected attempt's tab |
| A reply to the task | While the attempts work, it goes to every attempt that is still in the task |
| `Ctrl+X` | Stops all attempts |
| `herdr-projects pick <id> <letter>` | Keeps that attempt, for example `herdr-projects pick 12 B` |

## Research-only tasks

A [research-only task](research-tasks.md) can have attempts too. Each attempt writes one report, in the attempt's own
folder among the reports. The picker reads the reports instead of diffs.

| `attempts.pick` | The task's report |
|---|---|
| `reviewer` | The reviewer also writes one report from all of them, in the task's own folder: the best of each, what they agree on, and where they differ |
| `you` | The report of the chosen attempt |

## Related pages

- [Run several attempts](../how-to/run-several-attempts.md)
- [Several attempts](../explanation/attempts.md)
- [Settings](settings.md)
- [Plan settings](plan-settings.md)
- [Limits](limits.md)
- [Keys](keys.md)
