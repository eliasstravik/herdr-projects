# The fallback agent

This page tells when another agent takes over the work of a task, what the new agent gets, and why some cases stay
with you. To set a fallback agent, read [set a fallback agent](../how-to/set-a-fallback-agent.md).

## Why there is a fallback agent

An agent can stop before its work is done: it is out of allowance, it is logged out, or its provider is down. When many
agents run on subscriptions, this is the usual reason that the work of a night stops. A fallback agent takes over the
work in these cases.

The fallback costs nothing while all goes well. No extra agent runs until a hand-over.

## When a task is handed over

The working agent **cannot go on** in three cases:

1. **Its provider is down.** The model is at capacity, the servers are overloaded, a rate limit applies, or the
   connection dropped. These problems stop by themselves. Thus the board first waits and tells the same agent to
   continue, after 1, 2, 4 and 8 minutes ([a turn cut off by the provider](provider-errors.md)). The board hands over the
   task only when the provider also cuts off the fifth try.
2. **It is out of allowance or logged out.** This case also includes a model that is not there. It also includes a
   reply that was cut off for good (out of room, a refusal). Waiting does not fix these problems. Thus the board never tries again: it
   hands over the task at once. There is one exception: the board gave the agent a model, and its program refuses that
   model. Then the same program starts again on its own default model. The board hands over the task only when the
   program also refuses that model
   ([models](../reference/models.md#a-model-that-the-program-refuses-while-the-agent-works)).
3. **Its work still fails after its rounds.** A failed review goes back to the agent `review.rounds` times. When the
   review after the last of those rounds still fails, the next agent gets the work. The check of the repo works in the
   same way: it goes back `review.rounds` times, and the board hands over a check that fails after that. With
   `review.rounds 0`, nothing goes back by itself. Then the board also hands over nothing for a review or a check: the
   task waits for you.

Thus, for one turn that ends badly, the order is:
**the tries of the board (only for an error that stops by itself) → the next agent on `worker.fallback` → you.**

## What is not handed over

These cases are yours, as they were before the fallback:

- The agent stops and says that it waits for you.
- The agent stops without a summary.
- You stop the agent.
- There is a conflict with main.
- There is feedback on its pull request.
- The board repairs a merge that you approved.
- The task stopped at its limit on time or tokens (see [Limits](#limits-on-time-and-tokens)).

## What a hand-over is

The next agent on the list gets the task **in the same worktree**, on the same branch, in the same Herdr pane. The
board ends the program that cannot go on, and starts the next one where the first one was.

The board tells the new agent:

- the task, exactly as the first agent got it (the title, the description, the rules of the board);
- that it takes over, from whom and why, and that it must look at the current work and continue, not start again;
- **what was done until now**: the commits on the branch, whether there are uncommitted changes, and the last message
  of the agent before it;
- **the open findings**: what the last review found, if that review failed, or the output of the failed check.

The new agent:

- runs on the model that its entry names (`claude/claude-opus-5-5`), or else on the default model of its program;
- takes the `worker.effort` and the `worker.access` of the task, where its program takes them;
- starts with its own rounds: the rounds that the agent before it used do not count against it.

If you gave a yes to the earlier work, the board asks you for it again.

## Who is next

The next agent is the first on the list that obeys all of these conditions:

- It is not the agent on the task.
- It was not on the task since you last stepped in.
- It is installed on this machine.
- It can work under the access of the task. For example, `pi` cannot ask before it acts, so the board does not use it
  while `worker.access` is `ask`.

The board also does not use an agent that it cannot start. If the board can start no agent, the task stays exactly as
it was: the same agent, its failure, and the rounds it used.

**When the list is used up, the task waits for you as it does without a list.** You see the same "Agent stopped",
"Needs a login" or failed review. The task panel also shows a `Note` that tells which agents the board did not use,
and why.

## When you step in, and when the list changes

- When you step in (a reply, `Ctrl+R`, text that you type in the terminal of the agent), the agent on the task
  continues. The board can then go through the list again from the start, because an allowance can be back by then.
- When you delegate the task again, the board starts with `worker.agent` again.
- The board reads the setting at the time of the hand-over. Thus you can set it on a task that is under way. It applies
  from the next time that the agent cannot go on.

## The reviewer

`reviewer.fallback` works in the same way for the reviewer, in the cases where it applies:

- **Its provider is down** (after the five tries of the board), it is out of allowance or logged out, or its model is
  not there. Then the next agent on the list reviews, **as the same review**. The tries that the provider cut off are not
  reviews, and use no round.
- **Its program is not installed on this machine.** Then the first agent on the list that is installed reviews. This is
  a hand-over like the others (`claude took over the review from codex: codex is not installed here`). Without a list,
  the program of the working agent reviews, as before.

A reviewer that took over also reviews the later rounds of that task, until you step in. When the list is used up, the
review fails as it does without a list.

**One case does not apply to the reviewer**: "failed `review.rounds` times". A reviewer that reviewed and found faults
did its work. If the board replaces it until a reviewer passes the change, the board shops for a verdict.

A review that timed out, or that ended without a verdict two times, is a failed review as before, not a hand-over. (The
first time, the board starts the reviewer one more time.) Nothing shows that another program does better.

## Why a repo file cannot set the fallback

The file of a repo, `.herdr-projects.json`, holds its seven facts and no more. Thus code that an agent writes cannot
turn a hand-over on or off ([settings](settings.md)).

## The fallback and the other settings

### Several attempts

The board works on each attempt like a task of its own. Thus each attempt has its own fallback list, and a hand-over in
one attempt changes nothing in the others. If you choose an attempt that changed hands, the task keeps the agent that
took it over, its model and the hand-overs ([several attempts](attempts.md)).

The two functions are different. A hand-over is one agent after another, only on failure, and costs nothing when all
goes well. Attempts are side by side, always.

### Limits on time and tokens

The board does **not** hand over a task that stopped at its limit. The limit is your maximum for the cost of the task.
A token limit counts all the agents of the task together, and another agent only continues to spend. The task waits for
you ([limits](limits.md)).

## Related pages

- [Set a fallback agent](../how-to/set-a-fallback-agent.md)
- [The fallback settings](../reference/fallback-settings.md)
- [A turn cut off by the provider](provider-errors.md)
- [Agent programs](../reference/agent-programs.md)
- [Several attempts](attempts.md)
- [Limits](limits.md)
- [The code of the retries and the fallback](../contributing/fallback.md)
