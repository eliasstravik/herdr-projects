# Several attempts, and one kept

**A task can be done several times at once, and the best result kept.** "Best of three" (one agent, three tries) and a
"council" (different agents, and a judge) are the same thing: a number of attempts, who makes them, and who picks.

```
# best of three, by the usual agent; you pick
herdr-projects delegate 12 --attempts.by 3

# a council of claude, codex and pi, with a named judge
herdr-projects delegate 12 --attempts.by "claude, codex, pi" --attempts.pick reviewer --reviewer.agent claude/claude-opus-5-5

# two models for every task in one repo
herdr-projects set attempts.by "claude, codex/gpt-6.1-sol" --repo my-app

# which attempt to keep, when the task waits for you
herdr-projects pick 12 B
```

Or tell the [Coordinator](coordinator.md) in plain words: "do this one three times and let me choose", "have Claude,
Codex and pi each try it and let Opus judge".

## The two settings

| Setting | Values | Built in | Means |
|---|---|---|---|
| `attempts.by` | a count (`3`), or a list of agents (`claude, codex, pi`; a model after a slash; a name may repeat) | `1` | how many attempts are made at once, and by whom. A count means that many of `worker.agent` |
| `attempts.pick` | `you`, `reviewer` | `you` | who chooses among the attempts that passed the check |

- **At most 5.** Every attempt is a whole agent with its own worktree, takes one of `board.agents`' slots, and costs
  what the task costs done once. A count above 5 is refused, and the refusal names 5.
- They work like every other [setting](settings.md): looked up task → your entry for the repo → you → built in; set
  with `set` (with `--task`, `--repo` or neither) and `unset`; passed to `add` and `delegate`; held by an
  [automation](automations.md) (`--automation`); listed by `show` and on the task's settings page.
- **A repo's own file cannot set them**, so code an agent writes cannot switch on five attempts.
- `attempts.by` is read when the task starts, `attempts.pick` when its attempts are through. So you can still change the
  picker while they work.
- A task in no repo is always done once: there is no worktree to give each attempt.

## What happens

```
attempts fan out (attempts.by)
  each attempt: build ─► check
─► pick (attempts.pick; the reviewer's comparison)
─► the chosen one: review loop ─► push, pull request ─► merge gate (merge.by)
```

1. **Each attempt works on its own.** It has its own git worktree, its own branch (`work/12a-…`, `work/12b-…`) and its
   own tiled pane in the task's Herdr space (`Agent A`, `Agent B`; [a task's panes](tabs.md)). Its agent is told the task and
   nothing about the other attempts. Each uses a slot of `board.agents`: with three attempts and two free slots, the
   third waits and the row says so (`C claude waits for a slot`).
2. **The check filters.** Each attempt goes through the usual first steps: its work is committed, main is brought in,
   and the repo's check runs, going back to its agent up to `review.rounds` times when it fails. An attempt is out if its
   check still fails after that, or it stopped without a summary, was stopped, or its agent is gone. An attempt that asks
   you something waits for your answer like any agent, in its own pane.
3. **The picker chooses** among those that passed:
   - **None left:** the task failed (`No attempt passed`). Every attempt's worktree, branch and pane are kept to look at.
     Delegating it again starts over with fresh attempts and removes the old ones.
   - **One left:** it wins, and nobody is asked.
   - **Several, `attempts.pick you`:** the task waits in the state **pick one**. If there is a reviewer (`review.when` is
     not `never`), it first compares the attempts in a tiled pane named `Picker`: it reads each change, tries each one, says
     what is good and bad about each, and recommends one. You choose.
   - **Several, `attempts.pick reviewer`:** the reviewer compares them the same way, and the one it names goes on by
     itself. If it names none (it failed or timed out), or there is no reviewer, the task waits for your pick and says
     why.
4. **Only the chosen one is reviewed.** It becomes the task: its worktree, branch, agent and conversation are the task's
   from then on. It alone gets the full review loop, is pushed, gets a pull request and is merged, exactly like a task
   done once ([how work gets done](how-work-gets-done.md)). Its check does not run again on the same commit, unless
   `check.command` changed in between. Reviewing every attempt in full would multiply the most expensive step for work
   that is thrown away.
5. **After the pick,** the other attempts' panes close. Their worktrees and branches stay until the task is done, then go
   with it (`workspace.cleanup`). What a losing attempt committed goes with it. A worktree with any file that was never
   committed is kept, with its branch, and the task says where. That counts ignored files too (a draft, but also
   `node_modules` or a build's output): git cannot tell them apart, so nothing of the kind is removed.

With `merge.by itself` and `attempts.pick you`, the task still stops for your choice, then merges by itself. With
`attempts.pick reviewer` it runs through with nobody asked.

Nothing of two attempts is ever put together in the repo, and nobody races: every attempt finishes, and the best one is
kept, not the first.

## On the board

- **One row.** A tag after the task's name says how many attempts it has (`×3`). The line under it has a mark per
  attempt: `A claude ✓ · B codex working 60% · C pi ✗ check`. The [Watcher](watcher.md) follows each attempt's agent;
  the row's bar is their average.
- **Pick one.** A task waiting for your choice stands under **needs you** and says `Pick one of 3`, with the reviewer's
  recommendation when there is one. In Herdr's sidebar, the task's space says the same, and each attempt that passed
  says `pick one`.
- **The task panel** has a part **Attempts** (headed **Pick one of 3** while it waits): the reviewer's comparison and
  recommendation on top, then each attempt with its agent, where it stands, its time, its tokens, the size of its
  change, what the reviewer said of it, and a link to its diff (for a research-only task, to its report). `›` marks the
  selected attempt and `★` the reviewer's choice, or the one that was kept.
- **Keys.**
  - `Ctrl+N` moves to the next attempt.
  - `Ctrl+L`, the approve key, keeps the selected one. It asks first.
  - `Enter` goes into the selected attempt's pane.
  - A reply to the task while its attempts work goes to every attempt still in it. `Ctrl+X` stops them all.
- **The web Board** shows the same row and part, with the same keys as buttons (`Next attempt`, `Yes`).
- `herdr-projects show <id>` names each attempt's agent and the one chosen. `details <id>` has the attempts and the
  comparison.

## Research-only tasks

A [research-only task](research.md) takes attempts too: one report per attempt, each in the attempt's own folder among
the reports. The picker reads the reports instead of diffs.

- When the reviewer picks (`attempts.pick reviewer`), it is also asked to write **one report from all of them** into the
  task's own folder: the best of each, what they agree on, and where they differ. That is the task's report.
- When you pick, the chosen attempt's report is the task's.

## Beside the other settings

Attempts stand between the start and the review:

- A [plan](plan.md) is made and approved once, before the attempts fan out.
- A [fallback agent](fallback.md) takes over inside one attempt, one after another and only on failure. Attempts are side
  by side, and always.
- [Limits](limits.md): the attempts are one task. `limit.time` holds for each attempt's own runs, `limit.tokens` for all
  the attempts and the task together.
