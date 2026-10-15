# Several attempts, and one kept

A task can be done several times at once and one result kept. "Best of three" (one agent, three tries) and a
"council" (different agents, and a judge) are the same thing: a number of attempts, a list of who makes them, and
somebody who picks. It is two [settings](settings.md):

| Setting | Values | Built in | Means |
|---|---|---|---|
| `attempts.by` | a count (`3`), or a list of agents (`claude, codex, pi`; a model after a slash; a name may repeat) | `1` | how many attempts are made at once, and by whom. A count means that many of `worker.agent` |
| `attempts.pick` | `you`, `reviewer` | `you` | who chooses among the attempts that passed the check |

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

To the [Coordinator](coordinator.md) the same are plain words: "do this one three times and let me choose", "have
Claude, Codex and pi each try it and let Opus judge".

Both are settings like every other: looked up task → your entry for the repo → you → built in, set with `set` (with
`--task`, `--repo` or neither) and `unset`, passed to `add` and `delegate`, held by an [automation](automations.md)
(`--automation`), and listed by `show` and on the task's settings page. Neither may be said by a repo's own file, so
code an agent writes cannot switch on five attempts. `attempts.by` is read when the task starts and `attempts.pick`
when its attempts are through, so the picker can still be changed while they work. A task in no repo is always done
once: it has no worktree to give each attempt.

**At most 5.** Every attempt is a whole agent with its own worktree, holds one of `board.agents`' slots and costs
what the task costs when it is done once. A count above 5 is refused, and the refusal names 5.

## What happens

```
attempts fan out (attempts.by)
  each attempt: build ─► check
─► pick (attempts.pick; the reviewer's comparison)
─► the chosen one: review loop ─► push, pull request ─► merge gate (merge.by)
```

1. **Each attempt is on its own.** It has its own git worktree, its own branch (`work/12a-…`, `work/12b-…`) and its
   own tab in the task's one herdr space (`Agent A`, `Agent B`; [a task's tabs](tabs.md)). Its agent is told the task
   and nothing about the other attempts. Each one uses a slot of `board.agents`: with three attempts and two free
   slots, the third waits its turn and the row says so (`C claude waits for a slot`).
2. **The check filters.** Each attempt goes through the usual first steps by itself: its work is committed, main is
   brought in, and the repo's check runs, going back to its agent `review.rounds` times when it fails. An attempt
   whose check still fails after that is out. So is one that stopped without a summary, was stopped, or whose agent
   is gone. An attempt that asks you something waits for your answer like any agent, in its own tab.
3. **The picker chooses**, among those that passed:
   - **None left:** the task failed (`No attempt passed`), with every attempt's worktree, branch and tab kept to
     look at. Delegating it again starts over with fresh attempts and removes the old ones.
   - **One left:** it is the winner and nobody is asked.
   - **Several, `attempts.pick you`:** the task waits in the state **pick one**. Where there is a reviewer
     (`review.when` is not `never`) it first compares the attempts, in a tab named `Picker`: it reads each change,
     tries each one, says what is good and bad about each and recommends one. You choose.
   - **Several, `attempts.pick reviewer`:** the reviewer compares them the same way, and the one it names goes on by
     itself. If it names none (it failed, or timed out), or there is no reviewer, the task waits for your pick and
     says why.
4. **Review once.** The chosen attempt becomes the task: its worktree, branch, agent and conversation are the task's
   from then on. Only it gets the full review loop, is pushed, gets a pull request and is merged, exactly as a task
   done once ([how work gets done](how-work-gets-done.md)). Its check is not run a second time on the same commit, unless `check.command` was changed in between.
   Reviewing every attempt in full would multiply the dearest step for work that is thrown away.
5. **After the pick** the other attempts' tabs close. Their worktrees and branches stay until the task is done, and
   then go with it (`workspace.cleanup`). What a losing attempt committed goes with it; a worktree with any file
   that was never committed is kept, with its branch, and the task says where. That counts ignored files too (a
   draft, but also `node_modules` or a build's output): git cannot tell them apart, so nothing of the kind is removed.

`merge.by itself` with `attempts.pick you` still stops for your choice, and merges by itself after it. With
`attempts.pick reviewer` it runs through with nobody asked.

Nothing of two attempts is ever put together in the repo, and nobody races: every attempt finishes, and the best one
is kept, not the first.

## Research-only tasks

A [research-only task](research.md) takes attempts too: several reports, one per attempt, each in the attempt's own
folder among the reports. The picker reads the reports instead of diffs. When the reviewer is the one who picks
(`attempts.pick reviewer`), it is also asked to write **one report from all of them** into the task's own folder: the
best of each, what they agree on, and where they differ. That is the task's report. When you pick, the report of the
attempt you chose is the task's.

## On the board

- **One row.** A tag after the task's name says how many attempts it has (`×3`), and the line under it has a mark per
  attempt: `A claude ✓ · B codex working 60% · C pi ✗ check`. The [Watcher](watcher.md) follows each attempt's agent;
  the row's bar is their average.
- **Pick one.** A task that waits for your choice stands under **needs you**, and says `Pick one of 3`, with the
  reviewer's recommendation when there is one. In herdr's sidebar the task's space says the same line, and each
  attempt that passed says `pick one`.
- **The task panel** has a part **Attempts** (headed **Pick one of 3** while it waits): the reviewer's comparison
  with its recommendation on top, then each attempt with its agent, where it stands, its time, its tokens, the size
  of its change, what the reviewer said of it, and a link to its diff (for a research-only task, to its report).
  `›` marks the selected attempt and `★` the reviewer's choice, or the one that was kept.
- **Keys.** `Ctrl+N` moves to the next attempt. `Ctrl+L`, the approve key, keeps the selected one; it asks first.
  `Enter` goes into the selected attempt's tab. A reply to the task while its attempts work goes to every attempt
  that is still in it; `Ctrl+X` stops them all.
- **The web Board** shows the same row and the same part, and has the same keys as buttons (`Next attempt`, `Yes`).
- `herdr-projects show <id>` names each attempt's agent and the one that was chosen; `details <id>` has the
  attempts and the comparison.

## Beside the other settings

Attempts stand between the start and the review, so they fit with what else decides how a task runs: a plan would
be made and approved once, before the attempts fan out; a fallback agent would take over inside one attempt, one
after another and only on failure, where attempts are side by side and always; limits on time and tokens would hold
for the task as a whole, every attempt counted.
