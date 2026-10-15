# The gate

**The reviewer describes; the board decides.** The reviewer lists what is wrong, each finding with a severity. The
board then applies fixed rules: the work **passes** when no finding of its own is `major` or worse (`review.mustFix`).
Otherwise it goes back to the agent to fix, at most `review.rounds` times, and then waits for you.

## What the reviewer gives

No score and no merge advice. Every round it answers one fixed question, "Does the change do what the task asked?"
(`meets`), and lists its findings. Each finding has:

- a severity (below);
- a subject, and a [short title](titles.md) of it;
- where it is;
- a proof: what the reviewer actually ran or saw;
- whether it was there before the task (`preexisting`).

The severity says what the fault does:

| Severity | Means |
| --- | --- |
| `critical` | only one of four: a fault a user would hit in ordinary use; data that is lost or wrong; something unsafe; a requirement the task states plainly that is not built. A missing picture, a line of a document, or proof that cannot be had on this machine is not critical |
| `major` | a real fault in an unusual case; a claim that is not proven (a fix not shown to work, a test that cannot catch its bug); a document that states something false |
| `minor` | correct but could be better: wording, naming, a small inaccuracy, a narrow gap |
| `trivial` | a note; nothing to do |

## The board's rules

- **The reviewer's own findings decide.** `meets: false` adds a critical finding of the board's own ("The change does
  not do what the task asked") only when the reviewer listed nothing at all. Beside real findings, the board adds
  nothing. (That line used to be 67 of 292 findings, and 28 of 76 failed rounds had no critical finding of the task's
  own.)
- **A critical finding with an empty proof counts as major.**
- **A pre-existing finding never sends the work back**, and never becomes a task. A round whose findings are all
  pre-existing passes, whatever `meets` says. The review lists them as `pre-existing`, the agent is told to leave them,
  and nothing else is done. Add a task yourself for one you want fixed.
- **Pass:** no finding of the task's own at or above `review.mustFix` (`major` by default). A pass is what "may merge"
  means everywhere: a task with `merge.by: itself` merges ([merging by itself](#merging-by-itself)), an approved merge
  that was repaired goes through, and the board says `Pass`.
- **Fail:** the work goes back to the agent by itself, at most `review.rounds` times. Findings at or above the gate are
  sent as required, the rest as optional. When the cap is reached and it still fails, it stops and waits for you. With
  a fallback agent set (`worker.fallback`), the next agent on that list takes over first, with the open findings and
  rounds of its own ([the fallback agent](fallback.md)).
- **From the second round on**, the reviewer gets the earlier findings and only checks that each is fixed and that the
  fixes broke nothing. It may add a new finding only if it is critical, with proof. Anything smaller it notices late is
  left out (a pre-existing fault is still noted). A reply from you starts a full review again.
- **`review.rounds 0`** sends nothing back by itself: findings wait for you (`Ctrl+R` sends them). The review is still
  judged at `review.mustFix`, and a failed one never merges by itself.

Findings below the gate still show on the review page, in the task panel and in a comment on the pull request, marked
as not required.

## Merging by itself

By default, finished work waits for your yes (`merge.by: you`). With `merge.by: itself` it merges by itself the moment
it is green.

```
herdr-projects set merge.by itself                  every task
herdr-projects set merge.by itself --repo <name>    one repo's tasks
herdr-projects set merge.by itself --task <id>      one task (or: delegate <id> --merge.by itself)
herdr-projects unset merge.by                       take the choice back
```

Or tell the Coordinator "auto-merge this one" or "always auto-merge". The task panel shows it as `Merge`.

### What "green" means

The check passed, the review passed the gate (or the task is not reviewed), the branch is up to date with main with no
conflict, and the pull request has no failed check or requested change.

### Only the reviewed commit merges

The board remembers the commit the review saw and merges exactly that. A commit that reached the branch or the pull
request afterwards stops the merge, and is checked and reviewed first. (GitHub, asked right after the board's own push,
may still name the commit before it. That is not such a commit: while the branch at the remote is the reviewed one, the
board asks again.)

### Main moved since the review

The task catches up and merges, with nobody asked. The board merges main into the branch itself and runs the check on
the result.

- Clean merge and the check passes: the review still stands and the task merges at once. Such a merge of the board's
  own counts as reviewed. Any other new commit does not.
- The merge conflicts, or the check fails after it: the task goes back to its agent to repair. What the agent made of
  it is reviewed before it may merge, the conflicts it resolved included.

### One at a time

Every merge moves main under the tasks behind it, so a repo's tasks merge in a line: one catches up, is checked and
merges, then the next, in the order they became ready.

- A waiting task says `Waiting to merge: <id> goes first`.
- It is not brought up to date before its turn, so it catches up once, not once per merge ahead of it.
- A task that must be reviewed again leaves the line, and joins it again after.
- The line moves with the board closed too.

### What still stops and asks you

- a review that still fails the gate after its rounds;
- checks on GitHub that still fail after the rounds, or never finish (`merge.checks`);
- a merge that failed;
- a login, a question or a permission request from the agent;
- a checkout of yours on another branch;
- **a task you stepped into after it asked you** (a reply to its agent, a turn in its terminal, delegating it again)
  asks you again too, once. `set merge.by itself --task <id>` on that task says you do not need to be asked.

Sending the board's own words back to the agent (the review, or what went wrong: `Ctrl+R`, `review`, `fix`) is not
stepping in. You added nothing of your own, the repair is checked and reviewed, and it merges by itself when it passes.

An older version did count that as stepping in, and left such tasks waiting for a yes. Such a task is let go only where
the board can prove it from what its agent was told: its task once, before any review, and since then nothing but
reviews of its own, letter for letter as the board words them. With anything else in that conversation (a failure's
details sent back included: they are not kept, so they cannot be told from words of yours), or a conversation the
board cannot read (only Claude Code's can be), the task asks you once more. A hold made after the board stopped
counting a send-back is never lifted this way.

### It counts from the moment you change it

Also for tasks under way.

- **Turned on:** a running task merges when it turns green, and a green task already waiting for your yes merges at
  once. Turned on for every task, that is every such task: look at what is waiting first.
- **Turned off:** a task waiting to merge by itself (for GitHub's checks or a merge queue) goes back to waiting for your
  yes. So does one whose merge the board had started but not yet made: the setting is asked again right before the
  merge.

### A repo cannot switch it on for itself

A repo's own committed file cannot turn this on. `"land": "auto"` in `.herdr-projects.json` used to; that file now holds
seven facts only ([settings](settings.md)). So code an agent writes can never make its own work merge unasked. To merge
by itself in one repo, say so yourself: `set merge.by itself --repo <name>`. `show <id>` says what a task really runs
on, and where each value comes from.

## When the agent stops with an error or a question

- **An error, a usage limit, a cut-off or a refusal** is saved as "Agent stopped" with the real text. A provider error
  that passes by itself is retried first ([a turn cut off by the provider](provider-errors.md)).
- **A login error** makes the task "Needs a login", and that comes first. Older failure details are not sent, and
  pull-request feedback does not wake the agent, until you have logged in and then replied, pressed `Ctrl+R` or typed
  in the agent's terminal.
- **A question.** A turn only waits on you when the agent ran `report … --state blocked`. A final message that merely
  ends in a question goes on to check and review, and you still approve at the end
  ([an agent that waits for you is not finished](waiting-for-you.md)).
