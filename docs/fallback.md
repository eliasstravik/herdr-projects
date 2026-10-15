# The fallback agent: another agent takes over

**When an agent cannot go on (out of allowance, logged out, its provider down), another agent can take over.** With
many agents on subscriptions, that is the usual reason a night's work stops. Two settings:

- **`worker.fallback`**: who takes a task over when its working agent cannot go on.
- **`reviewer.fallback`**: who reviews instead when the reviewer cannot.

Each is a list of agents, tried in order, written like `worker.agent` with commas between them
(`codex, claude/claude-opus-5-5`: a model after a slash), or `none`. `none` is built in: without a list nothing changes.
**It costs nothing while all goes well**: nobody extra runs until a hand-over.

```
herdr-projects set worker.fallback "codex, claude/claude-opus-5-5"   for every task of yours
herdr-projects set worker.fallback codex --repo my-app               for one repo
herdr-projects set worker.fallback none --task 12                    not for this one
herdr-projects delegate 12 --worker.fallback pi --reviewer.fallback claude
```

## When a task is handed over

The working agent **cannot go on** in three cases:

1. **Its provider is down**: the model is at capacity, the servers are overloaded, a rate limit, a dropped connection.
   These pass by themselves, so the board first waits and tells the same agent to carry on, after 1, 2, 4 and 8 minutes
   ([a turn cut off by the provider](provider-errors.md)). Only when the fifth try is cut off too is the task handed
   over.
2. **It is out of allowance, or logged out**, or its model is not there, or its reply was cut for good (out of room, a
   refusal). Waiting does not fix these, so they are never retried: the task is handed over at once.
3. **Its work still fails after its rounds.** A failed review goes back to the agent `review.rounds` times. When the
   review after the last of those still fails, the next agent gets the work. Same for the repo's check: it goes back
   `review.rounds` times, and a check that fails after that is handed over. With `review.rounds 0` nothing goes back by
   itself, and nothing is handed over for a review or a check either: it waits for you.

So for one turn that ends badly, the order is: **the board's own tries (only for an error that passes) → the next agent
on `worker.fallback` → you.**

**Not handed over**, as before: when the agent stops and says it waits for you, when it stops without a summary, when you
stop it, for a conflict with main or feedback on its pull request, or while a merge you approved is being repaired.
Those are yours.

## What a hand-over is

The next agent on the list gets the task **in the same worktree**, on the same branch, in the same Herdr pane. The
program that cannot go on is ended, and the next one starts where it was. It is told:

- the task, exactly as the first agent got it (title, description, the board's rules);
- that it takes over, from whom and why, and to look at what is there and carry on, not start over;
- **what was done so far**: the commits on the branch, whether changes lie uncommitted, and the last message of the agent
  before it;
- **the open findings**: what the last review found, when that review failed, or the failed check's output.

The new agent:

- runs on the model its entry names (`claude/claude-opus-5-5`), else its program's own;
- takes the task's `worker.effort` and `worker.access`, where its program takes them;
- starts with rounds of its own: what the agent before it used up does not count against it.

A yes you had given to the earlier work is asked for again.

### Who is next

The first on the list that:

- is not the agent on the task;
- was not on it since you last stepped in;
- is installed on this machine;
- can work under the task's access (`pi` cannot ask before it acts, so it is passed over while `worker.access` is
  `ask`).

One that cannot be started is passed over too. When nobody could be started, the task is exactly as it was: the same
agent, its failure, the rounds it used.

**When the list is used up, the task waits for you as it would without a list**: the same "Agent stopped", "Needs a
login" or failed review, plus a `Note` in the task panel that says who was passed over and why.

### Stepping in, and changing the list

- When you step in (a reply, `Ctrl+R`, typing in the agent's terminal), the agent on the task carries on, and the list
  may be gone through again from the start: by then an allowance may be back.
- Delegating the task anew starts with `worker.agent` again.
- The setting is read at the moment of the hand-over. So you can set it on a task already under way, and it counts from
  the next time its agent cannot go on.

## The reviewer

`reviewer.fallback` works the same for the reviewer, where it applies:

- **Its provider is down** (after the board's own five tries), it is out of allowance or logged out, or its model is not
  there: the next one on the list reviews, **as the same review**. The tries that were cut off are not reviews, and use up
  no round.
- **Its program is not installed on this machine**: the first one on the list that is installed reviews. That is a
  hand-over like the others (`claude took over the review from codex: codex is not installed here`). Without a list, the
  working agent's program reviews, as before.

A reviewer that took over reviews the later rounds of that task too, until you step in. With the list used up, the
review fails as it does without one.

**One case does not apply to the reviewer**: there is no "failed `review.rounds` times" for it. A reviewer that reviewed
and found faults has done its work. Replacing it until one passes the change would be shopping for a verdict. A review
that timed out or ended without a verdict (twice: the first time it is started once more) is a failed review as before,
not a hand-over: nothing says another program would do better.

## What you see

- **The row** says the hand-over in plain words while the new agent gets going, until the Watcher writes its first line
  about it: `codex took over from claude: out of allowance until 17:00`. Its program column names the agent on it now.
- **The task panel** keeps every hand-over under `Status`, as `Took over` (beside the list: the latest; full screen: all
  of them, each with its age). `On it` and the `Agent` row are the agent on it now. A `Takes over` row under each of the
  two is its list. Each review says who made it once a reviewer took over.
- **The history**: `herdr-projects details <id>` lists every hand-over (who, from whom, why, when) beside every review,
  and says who is on it now.
- **Herdr**: the agent's tab is `Agent (codex)` and a reviewer's `Reviewer #2 (claude)` once one took over, and the
  sidebar names the agent: `agent codex · working`, `reviewer claude · under review`.

The reason is said from the provider's own words:

- `out of allowance until 17:00`, where they say when it comes back (`resets 5pm`, `try again at 5:00 PM`,
  `try again in 2 hours`), and `out of allowance` where they do not;
- `logged out`, `its model is not available`, `its provider is overloaded, still after 5 tries`;
- `its work still failed review after 3 rounds`, `the check still failed after 4 tries`;
- anything else, as the provider said it.

## A setting like every other

Both are work settings ([settings](settings.md)):

- looked up nearest answer first: the task → your entry for the repo → you → built in;
- set with `set`, taken away with `unset`, passed to `delegate` (and `add --delegate`) as `--worker.fallback <list>`;
- held by [automations](automations.md);
- listed by `show` and `show <id>` with where the value came from, and on the board's settings page (`Ctrl+E`:
  `Agent's fallback`, `Reviewer's fallback`).

A wrong name refuses the whole list, changes nothing and names the nearest right one. A list that only names the agent
that is on it anyway says that it does nothing.

**A repo's own file may not set them.** `.herdr-projects.json` holds its seven facts and no more, so code an agent
writes cannot switch a hand-over on or off ([settings](settings.md)).

## Beside the other settings

- **[Several attempts](attempts.md).** Each attempt is worked on like a task of its own, so each has the fallback list
  for itself, and a hand-over in one attempt changes nothing in the others. When an attempt that changed hands is the one
  chosen, the task keeps the agent that took it over, its model and the hand-overs. The two differ: a hand-over is one
  agent after another, only on failure, and costs nothing when all goes well; attempts are side by side, always.
- **[Limits on time and tokens](limits.md).** A task stopped at its limit is **not** handed over. The limit is your cap
  on what the task may cost, a token limit counts all its agents together, and another agent would only go on spending.
  It waits for you, as that page says.

## For contributors

The hand-over is one function (`hand_over` in `crates/flow/src/finish/hand_over.rs`) that any step can call with its
reason.

- `crates/core/src/store/handovers.rs`: what is kept on the task, the next on the list, the words;
- `crates/flow/src/finish/hand_over.rs`: the working agent's hand-over;
- `crates/flow/src/finish/review_fallback.rs`: the reviewer's;
- tests: `tests/fallback.rs`, `tests/fallback_more.rs` and `tests/fallback_settings.rs`.
