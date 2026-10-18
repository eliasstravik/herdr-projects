# Rules of the gate

This page lists the rules that the board applies to a review. It gives the severities, the pass and fail rules, the
settings, and what stops a merge by itself. For why the gate works this way, see [the gate](../explanation/gate.md).

## What the reviewer gives

The reviewer gives no score and no merge advice. In every round, it answers one fixed question: "Does the change do
what the task asked?" (`meets`). It also lists its findings. Each finding has:

- a severity (below);
- a subject, and a [short title](titles.md) of it;
- where it is;
- a proof: what the reviewer really ran or saw;
- whether it was there before the task (`preexisting`).

## Severities

The severity tells what the fault does.

| Severity | Means |
| --- | --- |
| `critical` | Only one of four: a fault that a user finds in ordinary use; data that is lost or wrong; something unsafe; a requirement that the task states clearly and that is not built. A missing picture, a line of a document, or proof that this machine cannot give is not critical. |
| `major` | A real fault in an unusual case; a claim that is not proven (a fix not shown to work, a test that cannot find its bug); a document that states something false. |
| `minor` | Correct, but it can be better: wording, naming, a small inaccuracy, a narrow gap. |
| `trivial` | A note. There is nothing to do. |

## The rules of the board

| Rule | What the board does |
| --- | --- |
| The findings of the reviewer decide | `meets: false` adds a critical finding of the board's own ("The change does not do what the task asked") only when the reviewer listed no findings at all. When there are real findings, the board adds nothing. |
| A critical finding with an empty proof | It counts as `major`. |
| A pre-existing finding | It never sends the work back, and it never becomes a task. A round whose findings are all pre-existing passes, whatever `meets` says. The review lists them as `pre-existing`, and the agent is told to leave them. Nothing more is done. |
| Pass | No finding of the task's own is at or above `review.mustFix` (`major` by default). |
| Fail | The work goes back to the agent by itself, at most `review.rounds` times. Findings at or above the gate go as required, the others as optional. If it still fails at the limit, the task stops and waits for you. |
| Fail, with `worker.fallback` set | Before the task waits for you, the next agent on that list takes over, with the open findings and rounds of its own ([the fallback agent](../explanation/fallback.md)). |
| From the second round | The reviewer gets the earlier findings. It checks only that each one is fixed and that the fixes broke nothing. It can add a new finding only if the finding is critical and has proof. It does not add a smaller fault that it sees late, but it still notes a pre-existing fault. |
| A reply from you | A full review starts again. |

A pass means "can merge" everywhere:

- a task with `merge.by: itself` merges ([merging by itself](../explanation/gate.md#merging-by-itself));
- an approved merge that was repaired goes through;
- the board says `Pass`.

Findings below the gate still show on the review page, in the task panel and in a comment on the pull request. They
are marked as not required.

## Settings

| Setting | Values | Default | What it does |
| --- | --- | --- | --- |
| `review.mustFix` | a severity | `major` | The lowest severity that fails the gate. |
| `review.rounds` | a number | 3 | How many times a failed review goes back to the agent by itself. With `0`, nothing goes back by itself: the findings wait for you, and `Ctrl+R` sends them. The review is still judged at `review.mustFix`, and a failed review never merges by itself. |
| `review.timeout` | a time, or `none` | `20m` | How long one review can take. After it, the review fails (`review timed out after 20m (review.timeout)`) and continues as any failed review does. |
| `merge.by` | `you`, `itself` | `you` | Who says yes to the merge. With `itself`, the work merges at the moment that it is green. The task panel shows it as `Merge`. |
| `merge.checks` | `github`, `none` | `github` | If the board waits for the checks on GitHub ([GitHub's checks](../explanation/how-work-gets-done-github-checks.md)). |

`limit.time` is a different setting. It stops any run, also the run of the agent, keeps its work and waits for you
([limits](../explanation/limits.md)).

## What "green" means

Work is green when all of these are true:

- the check passed;
- the review passed the gate, or the task has no review;
- the branch is up to date with main, with no conflict;
- the pull request has no failed check and no requested change.

## What still stops and asks you

A task with `merge.by: itself` still stops and asks you in these cases:

- a review that still fails the gate after its rounds;
- checks on GitHub that still fail after the rounds, or never finish (`merge.checks`);
- a merge that failed;
- a login, a question or a permission request from the agent;
- a checkout of yours on another branch;
- a task that you stepped into after it asked you. This means a reply to its agent, a turn in its terminal, or
  delegating it again. It asks you one more time. `set merge.by itself --task <id>` on that task tells the board that
  it does not have to ask you.

When you send the words of the board back to the agent, you do not step in. This is the review, or what went wrong:
`Ctrl+R`, `review`, `fix`.

## When the agent stops with an error or a question

| What happened | What the board does |
| --- | --- |
| An error, a usage limit, a cut-off or a refusal | It saves it as "Agent stopped", with the real text. A provider error that goes away by itself is tried again first ([a turn cut off by the provider](../explanation/provider-errors.md)). |
| A login error | The task becomes "Needs a login", and that comes first. Older failure details are not sent, and feedback on the pull request does not wake the agent, until you logged in and then replied, pressed `Ctrl+R` or typed in the terminal of the agent. |
| A question | A turn waits for you only when the agent ran `report … --state blocked`. A final message that only ends in a question goes on to the check and the review. You still approve at the end ([an agent that waits for you is not finished](../explanation/waiting-for-you.md)). |

## Related pages

- [The gate](../explanation/gate.md)
- [Merge work by itself](../how-to/turn-on-auto-merge.md)
- [How work gets done](../explanation/how-work-gets-done.md)
- [Settings](settings.md)
