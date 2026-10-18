# Handing text to an agent

This page tells how the board gives text to an agent, how it makes sure that the text was really sent, and why it
does this.

## What the board does

The board types everything that it gives an agent into the box of the agent. Then it checks that the text was really
sent. If the text was not sent, the board presses enter again. If that also fails, the task says **Not started**, and
you retry.

The board types these:

- the task, at the start;
- your replies;
- review findings;
- notices about conflicts and pull requests;
- "carry on", after a turn that was cut off;
- the brief of the reviewer;
- what you say to the Coordinator.

## What you see

| The row says | Means |
| --- | --- |
| `Starting` (or `About to continue`) | The board waits for the program to come up, or it checks that the text went in. |
| `Has not started yet: pressing enter again` | The text is in the box. The board presses enter again. |
| **Not started: its instructions sit unsent in its box** | The text is still in the box 40 seconds after the last enter. |

While it waits, the row never says `working` or `finishing`.

## A task shown as not started

- **Retry.** Press the "yes" key on the board (`Ctrl+L`), or run `herdr-projects retry <id>`. Retry stops the
  program of the agent, starts it again in the same pane and gives it the same text, word for word. The board stops
  the program first because its box can still hold the old text. If the board typed on top of it, it would send the
  text two times.
- **Or go into the agent and press enter yourself.** The board sees that the agent starts, and the error goes away.
- **A reply from the board** goes into the same box, and goes out together with the text that was there. The board
  keeps both. If that also does not start, retry gives all of it, in order.
- **A program that stopped** with typed text that it never took shows the same error at once: the text went with the
  program. A late report of a turn that it finished before that changes nothing. Retry starts it again with the text.
- **Retry never stops a program that is alive and has the text.** If the words are in the saved conversation of the
  agent after all, retry only clears the error. The program and the conversation stay the same. A turn that ended in
  the meantime goes to the finish line.

The last rule is true also for tasks that an older board marked. That version noted when the agent came to rest, not
when the text was typed. Thus, for such a task, the board reads the whole conversation of the running program.

### The reviewer and the Coordinator

The same rules apply to the reviewer and the Coordinator. If a reviewer never starts on its brief, that review fails
("the reviewer did not start"). Retry runs it again. The chat of the Coordinator names words that the Coordinator did
not take ("your message sits unsent in its box"). The board does not type them again.

## Why this exists

On a busy machine, the text of a task once stayed in the box of its agent for twenty minutes, while the board said
"working".

Herdr calls a program ready when it is the foreground process of the pane. That can be seconds before the program has
drawn anything. The board typed the task at that moment. A program that is still starting keeps typed text in its box,
but it loses the enter that follows. Herdr presses enter 0.3 seconds after the text. Nothing was sent, and nothing saw
it.

## How the board makes sure

1. **It waits until the program is up.** After it starts an agent, the board watches its pane. It waits until the pane
   shows something after the start command, and the screen stays still for a moment (a minute at most). Then it types.
   A normal start takes about half a second more. This is the same for every agent: the board reads the pane, and
   nothing is specific to a program.
2. **It looks again after it types.** It keeps the reading of the agent that Herdr gave just before the text went in.
   - An agent that works, has finished a turn, or has written to its saved conversation since then has taken the text.
   - An agent that is at its prompt exactly as before has not taken it. The board presses enter again after 5 seconds,
     then after 10, then after 20. It never types the text a second time.
   - The board also checks an agent that Herdr cannot classify (status `unknown`). Its counters and its saved
     conversation still tell whether it moved.
3. **It says so** on the row, as in the table above.

### A message typed while the agent is in a turn

Here, "the agent is working" proves nothing: the agent works in both cases. Thus the board waits until the agent is at
rest, and looks from there.

- An agent that took the message continues with it at once.
- An agent that lost the enter is at its prompt, and gets enter again.
- Until that is clear, the board does not send the turn that just ended to the finish line. That turn is not the answer
  to the message.
- The surest proof is the words in the saved conversation of the agent.
- An agent that never takes the message shows as not started. It stays so until you retry or press enter.

The board checks the messages of the Coordinator in the same way, on the tick of the board.

### A message that the agent takes during a turn

**A message that the agent takes during a turn counts as delivered at that time.** Some programs read what was typed
between two steps of a turn (Claude Code does). They write the words into the saved conversation at that moment, as a
note of its own kind (`queued_command`), not as a prompt. The board looks for these notes every few seconds while the
agent works. From then on, the message is delivered. The end of the turn, however much later, is the answer to it, and
goes to the finish line.

- The board reads back to the moment when the text was typed, however long the conversation is now. Before, it read
  only the last 768 KB. On one real task, the agent took a message early in a long turn, 3.5 MB before the end of the
  turn. The board did not find it, and pressed enter three times into an empty box. The task showed "not started",
  with its work done and saved.
- A note of a program that says that the words only wait in its queue does not count. The program has not read them
  yet.

## The waits

| Variable | Default | What |
| --- | --- | --- |
| `HERDR_PROJECTS_READY_MS` | 60000 | The longest time that the board waits for a started program to draw its screen. With 0, the board types at once. |
| `HERDR_PROJECTS_SUBMIT_MS` | 5000 | The wait before the first extra enter. Each later wait is two times the wait before it. |

The board gives very long text in a file:
[when an agent could not start](could-not-start.md#very-long-messages-go-in-a-file).

The code is `crates/flow/src/submit.rs`. What was measured on real programs is in
[what was checked on Herdr 0.9.3](../contributing/herdr-0.9.3-facts.md).

## Related pages

- [How work gets done](how-work-gets-done.md)
- [When an agent could not start](could-not-start.md)
- [Delegate a task and merge its work](../how-to/delegate-and-merge.md)
- [Troubleshooting](../how-to/troubleshoot.md)
