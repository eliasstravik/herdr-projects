# Limits: how long a run may take, how many tokens a task may use

The board lets work run, be reviewed and merge by itself, and on a schedule. Two settings put a cap on that. Both
are `none` until you set them, so nothing changes for a board that sets neither.

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

To the Coordinator the same is plain words: "never more than two hours on this one", "at most two million tokens a
task".

They are settings like every other ([settings](settings.md)): looked up nearest answer first (the task, your entry
for the repo, you, built in), listed by `show` and `show <id>` with the place the value came from, on the settings
page of a task and of an automation, and a refused value changes nothing and names the nearest right one (`2 hours`:
"Did you mean 2h?"). One thing they are not: **a repo's own `.herdr-projects.json` cannot hold them**, so code an
agent writes can neither set a limit nor lift one.

## What is measured

**A run** is one stretch of an agent's work: from the moment it is set going (by you, or by the board with a
failed check's output or a review's findings) until that turn ends. For the reviewer, a run is one review. The
clock is the board's own, looked at every couple of seconds by whoever keeps the board up to date (the keeper, or an
open board), so it works with the board closed. An agent that shows a dialog and waits for your answer is not
stopped for waiting.

**Tokens** are counted for the whole task: its agent, every review, every round, and any new chat its agent
started. The board reads them from each program's own saved conversation: everything the model read and everything
it wrote, every request added up, as the provider counts it. **What a request reads again from the provider's cache
counts too.** A request reads the whole conversation so far, so the number grows fast: fifty steps at 100k of
context are five million tokens, and a review is easily half a million. Choose the count with that in mind (`show
<id>` and the task panel say what a task has used). For Claude Code it is input, cache-write, cache-read and
output tokens; for Codex its running total of input and output; for pi input, cache-read, cache-write and output.

The count is made **at the end of each turn**, the agent's and the reviewer's. So one turn can pass the limit: a
task with `limit.tokens 5m` whose agent uses 6m in its first turn is stopped at 6m, not at 5m. A long turn is what
`limit.time` is for.

While no limit is said for a task the board reads no conversation at all: it only writes down which ones the task
had. The moment a limit is said, all of them are read, so a limit set late still counts the whole task, every
review that ended before included.

A program whose tokens the board cannot count adds nothing to the count, and `show <id>` says for it that
`limit.tokens` does nothing (`does nothing for <program>: its tokens cannot be counted`). The three programs whose
conversation the board reads (Claude Code, Codex, pi) can all be counted; herdr's other programs cannot
([agents](agents.md)).

**When each is read.** `limit.time` is read when a run starts and holds for that run: set or changed while a run
is under way, it counts from the next run (the next time the agent is set going, or the next review). `limit.tokens`
is read at the end of each turn, when the count is made. (The run's limit is kept on the task by the first look at
the run, or by the change itself, which keeps the old value for every run under way before it saves the new one:
also in a run's first seconds.)

## What happens at a limit

- The agent is stopped: its turn is interrupted, as the stop key does it. A reviewer's tab is closed.
- **Its work is kept as it is**: the worktree, the branch and the conversation stay. Nothing is committed for it,
  pushed or thrown away.
- **Nothing is merged.** No check and no review are started for it either.
- The task waits for you under "needs you", with one plain line: `stopped at its 2h limit`, or
  `stopped at 2m tokens`. herdr's sidebar says the same, `board.notify` tells you ("needs you"), and an automation
  that waits for `needs-you` fires.
- It never happens again by itself: a stopped task is looked at for no limit until somebody carries on.

Where the tokens run out decides what stops:

- *At the end of the agent's turn*: the task stops there, before the check and the review.
- *At the end of a review whose findings would go back to the agent*: the findings are kept on the task and do not
  go back by themselves.
- *At the end of a review that passed*: the task stops like any other at its limit, under "needs you", with what
  was reviewed kept. It is not put up as ordinary approval and it does **not** merge: not by itself (`merge.by`
  `itself`), and not as a merge you had approved that the board was repairing.
- *At the end of a review of work with nothing to land* (a report, work outside a repo): it stops as well,
  instead of ending as done; carrying on is what ends it.
- *A turn the provider cut off*, the agent's or the reviewer's, is not tried again by itself on a task at its limit.

## Carrying on

| You want | Do |
|---|---|
| it to continue | `Ctrl+R` on the task, or `herdr-projects retry <id>`, or reply to its agent with your own words |
| to give it more | set a higher limit (or `none`): `herdr-projects set limit.time 4h --task <id>` |
| to take the work as it is | `Ctrl+L`, or `herdr-projects finish <id>` |

- **Continue** is your word, so the task has its limit once more: the stopped agent is told to carry on where it
  left off, its run starts at zero, and the tokens count from where they stood. A review that was stopped is done
  again; a review whose findings were kept goes to the agent; work that was finished and reviewed is put up for
  your approval as it stood (and merges by itself where `merge.by` says so).
- **A higher limit** carries on by itself within a couple of seconds, with nobody asked again. There the count
  goes on: a run stopped at 2h and given 4h has two hours left, and a task stopped at 2m tokens and given 3m has one
  million left. (This is the one change of `limit.time` that the stopped run itself takes up.)
- **As it is** sends the stopped agent's work through the check and the review like any finished turn, and from
  there on it is an ordinary task: what `merge.by` says decides how it lands.

Stopping the task yourself (`Ctrl+X`, `herdr-projects stop <id>`) takes the limit's mark off: it is then simply
stopped.

## What the board shows

- **The task panel**, while an agent runs on a task with a limit, under Status: `Time 1h12m of 2h, this run` and
  `Tokens 1.2m of 2m`. From four fifths of a limit on they are amber. Under Settings: `Limits 2h a run · 2m tokens`.
  Under Session, full screen: `Used so far`, the tokens counted for the task (once a limit was said for it).
- **The row** says it from four fifths on, before what the agent is doing: `1h40m of its 2h · Running the tests`.
- **A stopped task** has a part of its own, "Stopped at a limit": what stopped it, what it had used, and the three
  ways on.
- **`details <id>`** has a `limits` entry: each limit, what has been used against it, and the stop when there was
  one. **`show <id>`** lists both settings with the place each value came from.

The web Board shows the same panel and rows, and has "Send back" and "Yes" as buttons for continue and as it is.
Its **Settings** button on a task opens that task's settings page: every work setting, the two limits among them,
with its plain label, its value and the place that comes from, what it may be and what it means; a row is changed
there ([the board in the web UI](web-ui.md)).

## Beside the other settings

**Several attempts** ([attempts](attempts.md)): a task done several times at once is one task to its limits. Each
attempt reads its task's limits; `limit.time` holds for each attempt's own runs, and `limit.tokens` for all the
attempts and the task together, whichever of them a turn ends in.

The research these come from (task herdr-projects/154) proposes more settings in the same list: a fallback
agent, a plan you approve. The limits are built to hold for those too: tokens are counted per conversation file and
added up on the task, so a planner's conversation is one more file; a run is whatever agent is at work in the
task's stage; and a stop is the board's ordinary stop with a mark on it (`limitStop`), which a fallback can read to
take over instead of waiting for you.

## Where it is

`crates/flow/src/limits/` (`run.rs`: the light part, a run and what counts; `spent.rs`: counting; `reached.rs`: the
stop; `tick.rs`: the pass that looks; `shown.rs`: what is shown), `crates/flow/src/actions/limits.rs` (continue, as it
is), `crates/core/src/agents/read_tokens.rs` (one counter per program). The token limit is asked at the end of a turn in
`crates/flow/src/finish/turn_ended.rs` and `crates/flow/src/finish/pipeline.rs`, and before anything merges by itself in
`crates/flow/src/finish/auto_merge.rs`. Tests: `tests/limits.rs`, `tests/limits_more.rs`, `tests/limits_tokens.rs`.
