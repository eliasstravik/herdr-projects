# The gate

This page tells why the board, not the reviewer, decides if work passes, and how a merge by itself stays safe. For
the list of rules and settings, see [rules of the gate](../reference/gate-rules.md).

## The reviewer describes, the board decides

The reviewer lists what is wrong, and gives each finding a severity. The board then applies fixed rules. The work
**passes** when no finding of its own is `major` or worse (`review.mustFix`). If it does not pass, it goes back to the
agent for a fix, at most `review.rounds` times. Then it waits for you.

The reviewer gives no score and no merge advice. It answers one fixed question: "Does the change do what the task
asked?" (`meets`). The severities and the fields of a finding are in
[rules of the gate](../reference/gate-rules.md#severities).

## Why the findings of the reviewer decide

`meets: false` adds a critical finding of the board's own only when the reviewer listed nothing at all. When there are
real findings, the board adds nothing. Before, that line of the board was 67 of 292 findings. Also, 28 of 76 failed
rounds had no critical finding of the task's own.

## Why a pre-existing finding never sends the work back

A fault that was there before the task is not the fault of the task. Thus a pre-existing finding never sends the work
back, and it never becomes a task. The agent is told to leave it. If you want it fixed, add a task for it yourself.

## Why later rounds are narrower

From the second round, the reviewer gets the earlier findings. It checks only that each one is fixed, and that the
fixes broke nothing. It can add a new finding only if the finding is critical and has proof. It does not add a
smaller fault that it sees late, but it still notes a pre-existing fault. A reply from you starts a full review again.

## Merging by itself

By default, finished work waits for your yes (`merge.by: you`). With `merge.by: itself`, it merges by itself at the
moment that it is green ([what "green" means](../reference/gate-rules.md#what-green-means)). To turn it on, see
[merge work by itself](../how-to/turn-on-auto-merge.md).

### Only the reviewed commit merges

The board remembers the commit that the reviewer saw, and it merges exactly that commit. A commit that arrives on the
branch or on the pull request after the review stops the merge. The board checks and reviews that commit first.

GitHub can still name the commit before the board's own push, if the board asks right after the push. That is not
such a commit. While the branch at the remote is the reviewed one, the board asks again.

### Main moved since the review

The task catches up and merges, and nobody is asked. The board merges main into the branch itself, and runs the check
on the result.

- If the merge is clean and the check passes, the review stays and the task merges at once. Such a merge of the board's
  own counts as reviewed. Any other new commit does not.
- If the merge conflicts, or the check fails after it, the task goes back to its agent for a repair. The board
  reviews what the agent made of it before it can merge. This includes the conflicts that the agent resolved.

### One at a time

Each merge moves main under the tasks behind it. Thus the tasks of a repo merge in a line. One task catches up, is
checked and merges. Then the next task does the same, in the order in which they became ready.

- A task that waits says `Waiting to merge: <id> goes first`.
- The board does not bring a task up to date before its turn. Thus the task catches up one time, not one time for each
  merge before it.
- A task that the board must review again leaves the line. It joins the line again after the review.
- The line moves also when the board is closed.

### Stepping in, and the words of the board

A task that you stepped into after it asked you asks you again, one time
([what still stops and asks you](../reference/gate-rules.md#what-still-stops-and-asks-you)).

Sending the words of the board back to the agent is not stepping in. These words are the review, or what went wrong
(`Ctrl+R`, `review`, `fix`). You added nothing of your own, and the board checks and reviews the repair. The task
merges by itself when it passes.

An older version did count that as stepping in, and such tasks waited for a yes. The board lets such a task go only
where it can prove it from what its agent was told. The agent must have got its task one time, before any review.
After that, it must have got only reviews of its own, letter for letter as the board words them.

In other cases, the task asks you one more time. These are the other cases:

- the conversation has anything else in it. This includes the details of a failure that were sent back: the board does
  not keep them, so it cannot tell them from words of yours;
- the board cannot read the conversation. It can read only the conversations of Claude Code.

The board never lifts this way a hold that it made after it stopped counting a send-back as stepping in.

### It counts from the moment that you change it

This is true also for tasks that are under way.

- **Turned on:** A running task merges when it becomes green. A green task that already waits for your yes merges at
  once. If you turn it on for every task, this is true for every such task.
- **Turned off:** Some tasks wait to merge by itself, for the checks on GitHub or for a merge queue. Such a task goes
  back to waiting for your yes. So does a task whose merge the board started but did not complete. The board reads the
  setting again just before the merge.

### A repo cannot turn it on for itself

The committed file of a repo cannot turn on a merge by itself. Before, `"land": "auto"` in `.herdr-projects.json` did
this. That file now holds only seven facts ([settings](../reference/settings.md)). Thus code that an agent writes can
never make its own work merge without your yes.

To merge by itself in one repo, you must say so yourself. `show <id>` tells what a task really runs on, and where each
value comes from.

## Related pages

- [Rules of the gate](../reference/gate-rules.md)
- [Merge work by itself](../how-to/turn-on-auto-merge.md)
- [How work gets done](how-work-gets-done.md)
- [Staying current with main](how-work-gets-done-main.md)
- [The fallback agent](fallback.md)
- [Limits](limits.md)
