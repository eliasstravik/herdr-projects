# An agent that waits for you is not finished

This page tells how the board knows if an agent waits for you or is finished. It also tells why a question never
ends a task.

## Waiting is not finishing

An agent can stop because it needs you: a question, a permission or a sign-in. Such an agent is waiting. It is not
finished. Nothing that follows a finish happens: no check, no review, no push, no pull request, no merge, no "done".

The row of the task says what the agent needs, with its mark (`?`, `!` or the sign-in mark). The row stays like that
until you answer.

This is true for every task, also for [research-only](../reference/research-tasks.md) tasks:

- however many times in a row the agent asks. An interview of fifteen questions is fifteen waits and no review;
- if a report or a closing note is already there, or not.

## How an agent says that it waits

An agent can stop in two ways. The board tells them apart by one thing only:

| It means | The agent | What follows |
| --- | --- | --- |
| **I am waiting for you** | runs `herdr-projects report <id> --state blocked --kind <question\|permission\|auth> --msg "<why>"`, then stops | Nothing. The task waits for your answer. |
| **I am finished** | stops, with a short summary | The finish line: check, review, your approval ([how work gets done](how-work-gets-done.md)). |

The board cannot read a question from a last message, a report or a closing note. An agent that asks in plain words
and stops counts as finished. The board checks and reviews its work as it is. Thus the rules of every agent say
clearly:

- that command is the only way to say "I am waiting";
- it is for **one stop**. Your answer comes as the next message of the agent and ends the wait. An agent that asks
  again must run the command again, every time. This is also true for each question of an interview;
- a stop without it means "finished", whatever the last message, the report or the closing note says;
- the agent must never run it to delay the review of finished work.

The command gives the agent this answer:

```
Reported: you are waiting, not finished, so no check and no review follow this stop.
```

## A report saved early

The result of a research-only task is a report. Saving the report finishes nothing. An agent can save an interim
report, add to it, and continue to ask. While the task waits, the report already has its link (**Report** in the task
panel), so you can read what it has so far. The task is finished when its agent stops without saying that it waits.

## Asking cannot prevent a review

A task that waits is never done, pushed or merged. The board checks and reviews the first turn that ends without the
command, as usual.

## A review that is already running

Sometimes a task becomes a waiting task while its review runs: its agent said so in a turn that began after the
review began. Then the board **stops** the review:

- the tab of the reviewer closes;
- the board keeps no verdict;
- nothing goes back to the agent as a failed round;
- the board counts no round.

A review of work whose agent still waits for an answer tells you nothing useful. The board reviews the work when it is
finished.

An agent can ask in plain words, without the command. The review that then runs is a real review of what was there.
The board keeps it and shows it. If you reply to the agent, the round counter starts again.

## How the board decides which turn waits

The agent says that it waits in the turn that it then ends. The end of that turn reaches the board two times, in no
fixed order:

- Herdr sees that the pane of the agent is at rest;
- the agent's own turn-end report starts the finish line.

Thus the board does not decide by which one saw the end first. It decides by the turn itself, which it reads from the
saved conversation of the agent:

- A turn that **began before** the agent said that it waits, and ended after, is the turn in which it waits. Its end
  is not a finish. This is true also when Herdr took a short change of the agent's status for a new turn in the
  meantime. The board puts the task back to waiting, with the kind and the words that the agent gave.
- A turn that **began after** it is your answer, in progress. You replied from the board, or typed in the agent's own
  terminal. Its end is a finish like any other, unless the agent says again that it waits.

Usually the board reads only the end of the saved conversation. A very long turn (a large tool result, or hours of
work) has its prompt further back. Then the board reads the whole file for it, that one time.

If the board cannot read the saved conversation at all, a task that still says that it waits continues to wait. The
board does not guess a finish. Reply from the board, and the board judges the next turn as usual.

## Related pages

- [How work gets done](how-work-gets-done.md)
- [Research tasks](../reference/research-tasks.md)
- [Rules of the gate](../reference/gate-rules.md)
- [Your first task](../tutorials/first-task.md)
