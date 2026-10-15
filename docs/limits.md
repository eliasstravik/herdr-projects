# Limits: time and tokens

**Two settings put a cap on what a task may cost.** When a task reaches one, its agent is stopped, its work is kept, and
it waits for you. Nothing is merged. Both are `none` until you set them, so a board that sets neither works as before.

| Setting | Values | Means |
|---|---|---|
| `limit.time` | a time (`2h`, `45m`, `90s`) or `none` | the longest one run of an agent may take |
| `limit.tokens` | a count (`2m`, `500k`, `1.5m`) or `none` | the most tokens a task may use, in all |

```
herdr-projects set limit.time 2h                      # for every task of yours
herdr-projects set limit.tokens 2m --repo my-app      # for one repo
herdr-projects set limit.time 45m --task my-app/12    # for one task, also while it is under way
herdr-projects delegate 12 --limit.time 2h --limit.tokens 5m
herdr-projects add "Tidy the docs" --limit.time 1h     # with or without --delegate
herdr-projects set limit.tokens 1m --automation "Nightly look"
herdr-projects unset limit.time --task my-app/12
```

Or tell the Coordinator: "never more than two hours on this one", "at most two million tokens a task".

They are settings like every other ([settings](settings.md)):

- looked up nearest answer first: the task, your entry for the repo, you, built in;
- listed by `show` and `show <id>` with where the value came from, and on the settings page of a task and of an
  automation;
- a refused value changes nothing and names the nearest right one (`2 hours`: "Did you mean 2h?");
- **a repo's own `.herdr-projects.json` cannot hold them**, so code an agent writes can neither set a limit nor lift one.

## What happens at a limit

- The agent is stopped: its turn is interrupted, as the stop key does. A reviewer's tab is closed.
- **Its work is kept as it is**: the worktree, the branch and the conversation stay. Nothing is committed, pushed or
  thrown away for it.
- **Nothing is merged**, and no check or review is started for it.
- The task waits for you under "needs you", with one plain line: `stopped at its 2h limit`, or `stopped at 2m tokens`.
  Herdr's sidebar says the same, `board.notify` tells you ("needs you"), and an automation that waits for `needs-you`
  fires.
- It never happens twice by itself: a stopped task is checked against no limit until somebody carries on.

Where the tokens run out decides what stops:

- **At the end of the agent's turn**: the task stops there, before the check and the review.
- **At the end of a review whose findings would go back to the agent**: the findings are kept on the task and do not go
  back by themselves.
- **At the end of a review that passed**: the task stops like any other at its limit, under "needs you", with what was
  reviewed kept. It is not put up for ordinary approval, and it does **not** merge: not by itself (`merge.by itself`),
  and not as a merge you had approved that the board was repairing.
- **At the end of a review of work with nothing to land** (a report, work outside a repo): it stops too, instead of
  ending as done. Carrying on is what ends it.
- **A turn the provider cut off**, the agent's or the reviewer's, is not retried by itself on a task at its limit.

## Carrying on

| You want | Do |
|---|---|
| it to continue | `Ctrl+R` on the task, or `herdr-projects retry <id>`, or reply to its agent with your own words |
| to give it more | set a higher limit (or `none`): `herdr-projects set limit.time 4h --task <id>` |
| to take the work as it is | `Ctrl+L`, or `herdr-projects finish <id>` |

- **Continue** is your word, so the task has its limit once more. The stopped agent is told to carry on where it left
  off; its run starts at zero, and the tokens count on from where they stood. A review that was stopped is done again. A
  review whose findings were kept goes to the agent. Work that was finished and reviewed is put up for your approval as
  it stood (and merges by itself where `merge.by` says so).
- **A higher limit** carries on by itself within a couple of seconds, with nobody asked. The count goes on: a run stopped
  at 2h and given 4h has two hours left; a task stopped at 2m tokens and given 3m has one million left. (This is the one
  change of `limit.time` that the stopped run itself takes up.)
- **As it is** sends the stopped agent's work through the check and review like any finished turn. From there it is an
  ordinary task: `merge.by` decides how it lands.

Stopping the task yourself (`Ctrl+X`, `herdr-projects stop <id>`) takes the limit's mark off: it is then simply stopped.

## What the board shows

- **The task panel**, while an agent runs on a task with a limit, under Status: `Time 1h12m of 2h, this run` and
  `Tokens 1.2m of 2m`. From four fifths of a limit on, they turn amber. Under Settings: `Limits 2h a run · 2m tokens`.
  Under Session, full screen: `Used so far`, the tokens counted for the task (once a limit was set for it).
- **The row** says it from four fifths on, before what the agent is doing: `1h40m of its 2h · Running the tests`.
- **A stopped task** has a part of its own, "Stopped at a limit": what stopped it, what it had used, and the three ways
  on.
- **`details <id>`** has a `limits` entry: each limit, what has been used against it, and the stop if there was one.
  **`show <id>`** lists both settings with where each value came from.

The web Board shows the same panel and rows, with "Send back" and "Yes" as buttons for continue and as it is. Its
**Settings** button on a task opens that task's settings page: every work setting, the two limits among them, with its
plain label, its value, where that comes from, what it may be and what it means. A row is changed there
([the board in the web UI](web-ui.md)).

## What is measured

### A run

One stretch of an agent's work: from the moment it is set going (by you, or by the board with a failed check's output or
a review's findings) until that turn ends. For the reviewer, a run is one review.

The clock is the board's own, looked at every couple of seconds by whoever keeps the board up to date (the keeper, or an
open board), so it works with the board closed. An agent that shows a dialog and waits for your answer is not stopped
for waiting.

### Tokens

Counted for the whole task: its agent, every review, every round, and any new chat its agent started. The board reads
them from each program's own saved conversation: everything the model read and wrote, every request added up, as the
provider counts it.

**What a request reads again from the provider's cache counts too.** A request reads the whole conversation so far, so
the number grows fast: fifty steps at 100k of context are five million tokens, and a review is easily half a million.
Choose the count with that in mind (`show <id>` and the task panel say what a task has used).

| Program | Counted |
|---|---|
| Claude Code | input, cache-write, cache-read and output tokens |
| Codex | its running total of input and output |
| pi | input, cache-read, cache-write and output |

- **The count is made at the end of each turn**, the agent's and the reviewer's. So one turn can pass the limit: a task
  with `limit.tokens 5m` whose agent uses 6m in its first turn is stopped at 6m, not 5m. A long turn is what
  `limit.time` is for.
- **While no limit is set for a task, the board reads no conversation at all**: it only writes down which ones the task
  had. The moment a limit is set, all of them are read, so a limit set late still counts the whole task, every review
  that ended before included.
- **A program whose tokens the board cannot count** adds nothing, and `show <id>` says for it that `limit.tokens` does
  nothing (`does nothing for <program>: its tokens cannot be counted`). The three programs whose conversation the board
  reads (Claude Code, Codex, pi) can all be counted; Herdr's other programs cannot ([agents](agents.md)).

### When each is read

- `limit.time` is read when a run starts, and holds for that run. Set or changed during a run, it counts from the next
  run (the next time the agent is set going, or the next review).
- `limit.tokens` is read at the end of each turn, when the count is made.
- (The run's limit is kept on the task by the first look at the run, or by the change itself, which keeps the old value
  for every run under way before it saves the new one: also in a run's first seconds.)

## Beside the other settings

- **[Several attempts](attempts.md)**: a task done several times at once is one task to its limits. Each attempt reads
  its task's limits. `limit.time` holds for each attempt's own runs; `limit.tokens` for all the attempts and the task
  together, whichever of them a turn ends in.
- **[A plan first](plan.md)**: tokens are counted per conversation file and added up on the task, so a planner's
  conversation is one more file. A run is whatever agent is at work in the task's current stage.
- **[The fallback agent](fallback.md)**: a stop at a limit is the board's ordinary stop with a mark on it (`limitStop`).
  A task stopped at its limit is not handed over: it waits for you.

These settings come from the research of task herdr-projects/154.

## Where it is

- `crates/flow/src/limits/`: `run.rs` (the light part: a run and what counts), `spent.rs` (counting), `reached.rs` (the
  stop), `tick.rs` (the pass that looks), `shown.rs` (what is shown);
- `crates/flow/src/actions/limits.rs`: continue, and as it is;
- `crates/core/src/agents/read_tokens.rs`: one counter per program;
- the token limit is checked at the end of a turn in `crates/flow/src/finish/turn_ended.rs` and
  `crates/flow/src/finish/pipeline.rs`, and before anything merges by itself in `crates/flow/src/finish/auto_merge.rs`;
- tests: `tests/limits.rs`, `tests/limits_more.rs`, `tests/limits_tokens.rs`.
