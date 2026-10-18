# The Watcher

This page tells what the Watcher does, when it looks at agents, and why it is built to cost little.

## What it is

**The Watcher is the small hidden model call behind the "doing now" line and the progress guess of each task**
([roles](../reference/roles.md)). It looks at agents about once a minute. It looks only at agents that are working and
whose screen changed. One call covers all agents. That keeps it cheap.

To change how often it looks, set `watcher.every` ([settings](../reference/settings.md)). The value is a time with its
unit, and the minimum is 5 seconds:

```
herdr-projects set watcher.every 120s
```

Each look starts a whole program: `claude -p`, `codex exec`, `pi -p`, or the one-shot mode of a different program
([agent programs](../reference/agent-programs.md)). Thus the Watcher is built to start as few programs as possible.

## When it looks

- **At most about once a minute for each working agent** (`watcher.every`). Before, it looked every 15 seconds.
- **Only at an agent that is working.** It does not look at an agent that is idle, that waits for you or that is
  finished.
- **Only when its screen changed.** The Watcher compares the words on the screen with the last look. A spinner, a timer
  or a token count that moved is not a change. Thus an agent that waits on a long command costs nothing.
- **At the reviewer during a review**, and only while Herdr says that the pane of the reviewer is working.
- **One call for all agents.** A call takes every agent whose minute is up, longest wait first, up to 8 in each call.
  The Watcher never looks at an agent before its minute is up. Agents can start at different times. To put them into
  the same call, a call waits while another agent is more than half-way to its minute. But no agent waits more than
  half as long again (90 seconds at the default). Thus six working agents cause one program start each minute, not six.
- **One call at a time, on the whole machine.** A second call never starts before the first one comes back. The turn
  is a small file in the data folder (`watcher-call.json`). Thus an open board and the keeper cannot both start a call
  at the same moment. If a process stops while it holds the turn, the turn becomes free in less than a minute. Calls
  start at least 15 seconds apart.
- **It looks less often when the machine is busy.** The pace follows the load (the one-minute load average for each
  core):
  - up to three quarters of the cores in use: the usual pace;
  - all cores in use: half as often;
  - from one and three quarter times what the machine can do: five times less often (one look in five minutes).
- **A call that fails waits longer.** Each failure in a row (no answer, a provider error) doubles the wait, up to eight
  times. The first answer brings back the usual pace.

The Watcher works the same when the board is open or closed. The open board and the keeper run the same code, one at a
time ([how work gets done](how-work-gets-done.md)).

## Between looks

The Watcher guesses only a percent. Each time the board draws, it calculates the minutes left from that percent and
the time spent. Thus the minutes continue to count between looks and do not jump.

The board sends the row itself to the sidebar of Herdr ([the sidebar](../reference/sidebar.md)). Thus the sidebar gets
the line, the percent and the minutes again as soon as one of them changes, with or without a look. The Watcher itself
writes nothing to Herdr.

## Before the first look

Each new run of an agent starts with no guess. A new run is an agent that started, was told to continue, or was sent
back after a review, a conflict or a retry. The first look takes about a minute. During that time, the line of the
task is never a bare "…":

- **If it had a sentence before:** that sentence stays, with its age in front (`12m ago: Running the check`). Thus you
  do not read it as new. The percent starts again at 0.
- **If it is its first run:** the line says `Just started`, or `Review just started` for a reviewer (each round has a
  new reviewer). If there is still no sentence after three minutes (because the calls of the Watcher fail), the line
  says `No status yet`.
- **If the Watcher is off,** no sentence comes and the line stays empty.

The prompt of the Watcher shows it some example sentences (`Fixing the two failing tests`,
`Waiting for the PR review on GitHub`). The reporting instructions of the agents show them too. The board shows what
they write exactly as written. The list, the task panel, the whole-task page and the sidebar of Herdr all show the same
words.

## A lighter start

When Claude is the Watcher (the default):

- It starts with nothing of yours loaded: no MCP servers, plugins, hooks, skills or `CLAUDE.md`
  (`--safe-mode --strict-mcp-config --disable-slash-commands`). It gets a two-line instruction in place of the long
  instruction of Claude Code. Login works as usual.
- If a Claude is too old to know these options, the Watcher starts it the plain way from then on.
- The prompt is smaller: the last 30 screen lines (3000 characters) in place of 40 (5000). It also has 800 characters
  of the description of the task in place of 1500.

Codex and pi start as before. The options of pi to load less saved nothing when measured.

## Measured

These numbers come from the 8-core machine that this was built on. It used Claude Code 2.1.296 and `claude-haiku-5-5`,
with a load average of about 9.5 during the measurement. The numbers are the processor time of the program itself.

| | One call | Six working agents, a minute |
|---|---|---|
| Before: every 15 seconds, a call per agent, up to 4 at once | 1.55 s of processor time, 5.1 s in all | 24 calls: about 37 s of processor time |
| Now: once a minute, one call for all, lighter start | 0.9 s, 2.6 s in all (six agents in one call: 1.1 s, 4.6 s) | 1 call: about 1.1 s of processor time |
| Now, on a machine at twice its capacity | the same | a call every five minutes: about 0.2 s a minute |

That is about a thirtieth of the processor time. Also, never more than one Watcher program runs. For comparison, these
are the plain one-shot calls of the other programs: `codex exec` 1.4 s of processor time, `pi -p` 2.0 s.

## Related pages

- [Roles](../reference/roles.md)
- [Settings](../reference/settings.md)
- [Change a setting](../how-to/change-a-setting.md)
- [Agent programs](../reference/agent-programs.md)
- [How work gets done](how-work-gets-done.md)
- [The sidebar](../reference/sidebar.md)
- [The Coordinator](coordinator.md)
