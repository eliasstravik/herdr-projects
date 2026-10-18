# Limits

This page gives the facts about the two limits on a task: time and tokens. It gives the values, what occurs at a limit,
how to continue, what the board shows and what the board counts.

## The two settings

| Setting | Values | Means |
|---|---|---|
| `limit.time` | a time (`2h`, `45m`, `90s`) or `none` | the longest time that one run of an agent can take |
| `limit.tokens` | a count (`2m`, `500k`, `1.5m`) or `none` | the most tokens that a task can use, in all |

Both are `none` until you set them.

They are settings the same as all others ([settings](settings.md)):

- The board looks them up nearest answer first: the task, your entry for the repo, you, built in.
- `show` and `show <id>` list them with where the value came from. So does the settings page of a task and of an
  automation.
- A value that the board refuses changes nothing. The board names the nearest correct value (`2 hours`:
  "Did you mean 2h?").
- **A repo's own `.herdr-projects.json` cannot hold them.** Thus code that an agent writes can neither set a limit nor
  remove one.

### When the board reads them

| Setting | Read | Effect |
|---|---|---|
| `limit.time` | When a run starts | It holds for that run. If you set or change it during a run, it counts from the next run: the next time the agent is set going, or the next review |
| `limit.tokens` | At the end of each turn | When the board makes the count |

The board keeps the run's limit on the task. It does so at its first look at the run, or at the change itself. A
change keeps the old value for every run under way before it saves the new one, also in the first seconds of a run.

## What occurs at a limit

- The board stops the agent: it interrupts its turn, as the stop key does. It closes a reviewer's tab.
- **Its work stays as it is**: the worktree, the branch and the conversation. Nothing is committed, pushed or thrown
  away for it.
- **Nothing is merged.** No check or review starts for it.
- The task waits for you under **needs you**, with one plain line: `stopped at its 2h limit`, or `stopped at 2m tokens`.
  Herdr's sidebar says the same. `board.notify` tells you ("needs you"). An automation that waits for `needs-you`
  fires.
- It never occurs two times by itself. The board checks a stopped task against no limit until somebody continues it.

### Where the tokens run out

| The tokens run out | What stops |
|---|---|
| At the end of the agent's turn | The task stops there, before the check and the review |
| At the end of a review whose findings would go back to the agent | The findings stay on the task. They do not go back by themselves |
| At the end of a review that passed | The task stops like any other at its limit, under **needs you**, and what was reviewed stays. It is not put up for ordinary approval. It does **not** merge: not by itself (`merge.by itself`), and not as a merge that you had approved that the board was repairing |
| At the end of a review of work with nothing to land (a report, work outside a repo) | It stops too, instead of ending as done. When you continue it, it ends |
| A turn that the provider cut off, the agent's or the reviewer's | The board does not retry it by itself on a task at its limit |

## Carrying on

| You want | Do |
|---|---|
| it to continue | `Ctrl+R` on the task, or `herdr-projects retry <id>`, or reply to its agent with your own words |
| to give it more | set a higher limit (or `none`): `herdr-projects set limit.time 4h --task <id>` |
| to take the work as it is | `Ctrl+L`, or `herdr-projects finish <id>` |

- **Continue** is your word, so the task has its limit one more time.
  - The stopped agent is told to continue from where it stopped.
  - Its run starts at zero. The tokens count on from where they were.
  - A review that was stopped is done again.
  - A review whose findings stayed on the task goes to the agent.
  - Work that was finished and reviewed is put up for your approval as it was. It merges by itself where `merge.by`
    says so.
- **A higher limit** continues by itself within a couple of seconds, and nobody is asked. The count goes on:
  - a run stopped at 2h and given 4h has two hours left;
  - a task stopped at 2m tokens and given 3m has one million left.
  - This is the one change of `limit.time` that the stopped run itself takes up.
- **As it is** sends the stopped agent's work through the check and the review, like any finished turn. From there it
  is an ordinary task: `merge.by` decides how it lands.

If you stop the task yourself (`Ctrl+X`, `herdr-projects stop <id>`), the limit's mark goes. The task is then simply
stopped.

## What the board shows

| Where | What it shows |
|---|---|
| The task panel, under Status, while an agent runs on a task with a limit | `Time 1h12m of 2h, this run` and `Tokens 1.2m of 2m`. From four fifths of a limit on, they turn amber |
| The task panel, under Settings | `Limits 2h a run · 2m tokens` |
| The task panel, under Session, full screen | `Used so far`: the tokens counted for the task (when a limit was set for it) |
| The row | From four fifths on, before what the agent does: `1h40m of its 2h · Running the tests` |
| A stopped task | A part of its own, "Stopped at a limit": what stopped it, what it had used, and the three ways to continue |
| `details <id>` | A `limits` entry: each limit, what was used against it, and the stop if there was one |
| `show <id>` | Both settings, with where each value came from |
| `show <id>`, for a program whose tokens the board cannot count | `does nothing for <program>: its tokens cannot be counted` |

On the web board:

- The panel and the rows are the same.
- "Send back" and "Yes" are the buttons for continue and for as it is.
- The **Settings** button on a task opens that task's settings page. It shows every work setting, the two limits among
  them. For each, it shows its plain label, its value, where that value comes from, what it can be and what it means.
  You change a row there ([the web UI](web-ui.md)).

## What the board counts

| Term | Means |
|---|---|
| A run | One stretch of an agent's work: from when it is set going (by you, or by the board with a failed check's output or a review's findings) until that turn ends. For the reviewer, a run is one review |
| Tokens | All tokens of the task: its agent, every review, every round, and any new chat that its agent started |

The board reads the tokens from each program's own saved conversation: all that the model read and wrote, every request
added up, as the provider counts it. Tokens that a request reads again from the provider's cache count too.

| Program | Counted |
|---|---|
| Claude Code | input, cache-write, cache-read and output tokens |
| Codex | its running total of input and output |
| pi | input, cache-read, cache-write and output |

Herdr's other programs cannot be counted ([agent programs](agent-programs.md)). For them, `limit.tokens` does nothing.

## Limits with other settings

| Setting | Effect on limits |
|---|---|
| [Several attempts](attempts-settings.md) | Each attempt reads its task's limits. `limit.time` holds for the runs of each attempt. `limit.tokens` holds for all the attempts and the task together, in whichever of them a turn ends |
| [A plan first](plan-settings.md) | A planner's conversation is one more conversation file, added up on the task. A run is the agent at work in the task's current stage |
| [The fallback agent](../explanation/fallback.md) | A stop at a limit is the board's ordinary stop, with a mark on it (`limitStop`). A task stopped at its limit is not handed over: it waits for you |

## Related pages

- [Set limits on time and tokens](../how-to/set-limits.md)
- [How limits work](../explanation/limits.md)
- [Settings](settings.md)
- [Attempts settings](attempts-settings.md)
- [Plan settings](plan-settings.md)
- [Limits code](../contributing/limits-code.md)
