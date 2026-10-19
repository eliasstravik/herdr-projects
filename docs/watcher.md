# The Watcher

**The Watcher is the small hidden model call behind each task's "doing now" line and its progress guess**
([roles](roles-and-commands.md)). It looks about once a minute, only at agents that are working and whose screen
changed, and one call covers every agent. That keeps it cheap.

Change how often with `watcher.every` ([settings](settings.md)), a time with its unit, at least 5 seconds:

```
herdr-projects set watcher.every 120s
```

Each look starts a whole program (`claude -p`, `codex exec`, `pi -p`, or another program's one-shot mode:
[agents](agents.md)), so it is built to start as few as it can.

## When it looks

- **At most about once a minute per working agent** (`watcher.every`). It used to be every 15 seconds.
- **Only at an agent that is working.** One that is idle, waits for you or has finished is not looked at.
- **Only when its screen changed.** The words on screen are compared with the last look. A spinner, a timer or a token
  count that moved is not a change, so an agent waiting on a long command costs nothing.
- **At the reviewer during a review**, and only while Herdr says the reviewer's own pane is working.
- **One call for everyone.** A call takes every agent whose minute is up, longest wait first, up to 8 per call. Nobody is
  looked at before its minute is up. To bring agents that started at different moments into the same call, a call
  holds off while another agent is more than half-way to its minute, but nobody waits more than half as long again (90
  seconds at the default). So six working agents are one program start a minute, not six.
- **One call at a time, on the whole machine**, never a second before the first comes back. The turn is a small file in
  the data folder (`watcher-call.json`), so an open board and the keeper ticking at the same moment cannot both start a
  call. A process that died holding the turn frees it within a minute. Calls start at least 15 seconds apart.
- **It gives way when the machine is busy.** The pace follows the load (the one-minute load average per core):
  - up to three quarters of the cores in use: the usual pace;
  - all cores in use: half as often;
  - from one and three quarter times what the machine can do: five times less often (one look in five minutes).
- **A failing call backs off.** Each failure in a row (no answer, a provider error) doubles the wait, up to eight times.
  The first answer brings back the usual pace.

It works the same with the board open or closed: the open board and the keeper run the same code, one at a time
([how work gets done](how-work-gets-done.md)).

## Between looks

The Watcher guesses only a percent. The board works out the minutes left from that percent and the time spent, each
time it draws, so they keep counting between looks and do not jump.

Herdr's sidebar is sent the row itself ([Herdr's sidebar](sidebar.md)), so it gets the line, the percent and the
minutes afresh as soon as one of them changes, look or no look. The Watcher itself writes nothing to Herdr.

## Before the first look

Every new run of an agent (started, told to go on, sent back after a review, a conflict or a retry) starts with no
guess. The first look takes about a minute. Meanwhile the task's line is never a bare "…":

- **It had a sentence before:** that sentence stays, with its age in front (`12m ago: Running the check`), so it is not
  read as new. The percent starts again at 0.
- **Its first run:** `Just started`, or `Review just started` for a reviewer (a new agent every round). After three
  minutes with still no sentence (the Watcher's calls fail), it says `No status yet`.
- **With the Watcher off,** no sentence comes and the line stays empty.

The Watcher's prompt shows it a few example sentences (`Fixing the two failing tests`,
`Waiting for the PR review on GitHub`), and so do the agents' instructions for reporting. What they write is shown
exactly as written. The list, the task panel, the whole-task page and Herdr's sidebar all say the same words.

## Default: off

The Watcher is off by default. The configured coding CLI pool runs in visible Herdr panes; none of those CLIs is
started as a hidden one-shot to produce status text. To opt into the existing helper, choose a compatible program
outside that pool explicitly, for example `herdr-projects set watcher.agent pi`. The supported helper programs and
their one-shot commands are listed in [agents](agents.md).

## Measured

Historical measurement from the earlier Claude-default version, on the 8-core machine used for that measurement
(Claude Code 2.1.296, `claude-haiku-5-5`, load average about 9.5), processor time of the program itself:

| | One call | Six working agents, a minute |
|---|---|---|
| Before: every 15 seconds, a call per agent, up to 4 at once | 1.55 s of processor time, 5.1 s in all | 24 calls: about 37 s of processor time |
| Now: once a minute, one call for all, lighter start | 0.9 s, 2.6 s in all (six agents in one call: 1.1 s, 4.6 s) | 1 call: about 1.1 s of processor time |
| Now, on a machine at twice its capacity | the same | a call every five minutes: about 0.2 s a minute |

That is about a thirtieth of the processor time, and never more than one Watcher program running. For comparison, the
other programs' plain one-shot calls: `codex exec` 1.4 s of processor time, `pi -p` 2.0 s.
