# Handing text to an agent

**Everything the board gives an agent is typed into its box, and the board checks it was really sent.** If it was not,
the board presses enter again. If that still fails, the task says **Not started** and you retry.

What gets typed: the task at the start, your replies, review findings, conflict and pull-request notices, "carry on"
after a cut-off turn, the reviewer's brief, and what you say to the Coordinator.

## What you see

| The row says | Means |
|---|---|
| `Starting` (or `About to continue`) | the board is waiting for the program to come up, or checking the text went in |
| `Has not started yet: pressing enter again` | the text sits in the box; the board presses enter again |
| **Not started: its instructions sit unsent in its box** | still sitting there 40 seconds after the last enter |

While it waits, the row never says `working` or `finishing`.

## A task shown as not started

- **Retry**: the "yes" key on the board, or `herdr-projects retry <id>`. It ends the agent's program, starts it again in
  the same pane and hands it the same text, word for word. The program is ended first because its box may still hold
  the old text, and typing on top would send it twice.
- **Or go into the agent and press enter yourself.** The board sees it start, and the error goes away.
- **A reply from the board** is typed into the same box and goes out together with what sat there. The board keeps
  both: if that does not start either, retry hands over all of it, in order.
- **A program that ended** with typed text it never took shows the same error at once: the text went with the program.
  A late report of a turn it finished before that changes nothing. Retry starts it again with the text.
- **Retry never ends a program that is alive and has the text.** If the words turn out to be in the agent's saved
  conversation after all, retry just clears the error: same program, same conversation, and a turn that ended
  meanwhile goes to the finish line. This holds for tasks an older board marked too. That version noted when the agent
  came to rest, not when the text was typed, so for such a task the whole conversation of the running program is read.

**The reviewer and the Coordinator** are held to the same. A reviewer that never starts on its brief fails that review
("the reviewer did not start"); retry runs it again. Words the Coordinator did not take are named in its chat ("your
message sits unsent in its box"). They are not typed again.

## Why this exists

On a busy machine, a task's text once sat in its agent's box for twenty minutes while the board said "working".

Herdr calls a program ready as soon as it is the pane's foreground process. That can be seconds before it has drawn
anything. The board typed the task at that moment. A program still starting keeps typed text in its box but drops the
enter that follows (Herdr presses it 0.3 seconds after the text). Nothing was sent, and nothing noticed.

## How the board makes sure

1. **It waits until the program is up.** After starting an agent, the board watches its pane until something is drawn
   after the start command and the screen holds still for a moment (a minute at most). Then it types. A normal start
   costs about half a second more. This is the same for every agent: it reads the pane, nothing program-specific.
2. **It looks again after typing.** It keeps Herdr's reading of the agent from just before the text went in.
   - An agent that is working, has finished a turn, or has written to its saved conversation since has taken the text.
   - One that sits at its prompt exactly as before has not. The board presses enter again after 5 seconds, then 10,
     then 20. It never types the text a second time.
   - An agent Herdr cannot classify (status `unknown`) is checked too: its counters and saved conversation still say
     whether it moved.
3. **It says so** on the row, as in the table above.

### A message typed while the agent is mid-turn

Here "the agent is working" proves nothing: it works either way. So the board waits until the agent is at rest, and
looks from there. An agent that took the message carries straight on with it. One that lost the enter sits at its
prompt and gets enter again. Until that is settled, the turn that just ended is not sent to the finish line: it is not
the answer to the message. The surest proof is the words standing in the agent's saved conversation. An agent that
never takes the message is shown as not started, and stays so until you retry or press enter. The Coordinator's
messages are checked the same way, on the board's tick.

**A message the agent takes mid-turn counts as delivered then.** Some programs read what was typed between two steps
of a turn (Claude Code does). They write the words into the saved conversation at that moment, as a note of its own
kind (`queued_command`), not as a prompt. The board looks for these every few seconds while the agent works. From then
on the message is delivered: the turn's end, however much later, is the answer to it and goes to the finish line.

- The board reads back as far as the moment the text was typed, however long the conversation has grown since. It used
  to read only the last 768 KB: a message taken early in a long turn (3.5 MB before its end, on one real task) was not
  found, enter was pressed three times into an empty box, and the task stood as "not started" with its work done and
  saved.
- A program's note that the words only wait in its queue does not count: it has not read them yet.

## The waits

| Variable | Default | What |
|---|---|---|
| `HERDR_PROJECTS_READY_MS` | 60000 | the longest the board waits for a started program to draw its screen; 0 types at once |
| `HERDR_PROJECTS_SUBMIT_MS` | 5000 | the wait before the first extra enter; each later wait is twice the one before |

Very long text is handed over in a file instead: [when an agent could not start](could-not-start.md#long-rules-and-long-messages).

The code is `crates/flow/src/submit.rs`. What was measured on real programs is in [checked on Herdr](checked-on-herdr.md).
