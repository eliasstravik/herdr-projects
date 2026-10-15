# The Watcher: how often it looks, and what it costs

The Watcher is the small hidden model call behind the "doing now" line and the progress guess
([roles](roles-and-commands.md)). Each look starts a whole program (`claude -p`, `codex exec`, `pi -p`, or another program's one-shot:
[agents](agents.md)), so it is
built to start as few of them as it can.

## When it looks

- **About once a minute per working agent**, at the most: the `watcher.every` [setting](settings.md), a time with
  its unit (`herdr-projects set watcher.every 120s`; 5 seconds at the least). It was every 15 seconds.
- **Only at an agent that works.** One that is idle, waits for you or has finished is not looked at.
- **Only when its screen changed.** The words on the screen are compared with the last look's. A spinner, a timer or
  a token count that moved is not a change, so an agent that waits on a long command costs nothing.
- **Only at a reviewer that works.** While a task is under review the Watcher follows the reviewer, and only while
  herdr says the reviewer's own pane is working.
- **One call for everyone.** A call takes every agent whose minute is up, the longest wait first, up to 8 in one
  call. Nobody is looked at before its minute is up. So that agents which started at different moments end up in
  the same call, a call holds off while another agent is more than half-way to its own minute, but nobody waits
  more than half as long again (90 seconds at the default): six working agents are one program start a minute, not
  six.
- **One call at a time, on the whole machine**, and never a second one while the first has not come back. The turn
  is a small file in the data folder (`watcher-call.json`), so an open board and a keeper that tick at the same
  moment cannot both start a call; a process that died with the turn frees it within a minute. From one call's
  start to the next there are at least 15 seconds.
- **It gives way first.** The pace follows the machine's load (the one-minute load average per core): up to three
  quarters of the cores in use, the usual pace; all cores in use, half as often; from one and three quarter times
  what the machine can do, five times less often (one look in five minutes).
- **A call that fails is not hammered.** Each failure in a row (no answer, a provider error) doubles the wait, up to
  eight times; the first answer brings the usual pace back.

It is the same with the board open and with it closed: the open board and the keeper run the same code, and only
one of them runs it at a time ([how work gets done](how-work-gets-done.md)).

## Between looks

The Watcher guesses only a percent. The board works out the minutes left from that percent and the time spent, every
time it draws, so they keep counting between looks and do not jump. herdr's own sidebar is told the row
itself ([herdr's sidebar](sidebar.md)), so it has the line, the percent and the minutes worked out afresh as soon as
one of them changes, look or no look. The Watcher writes nothing to herdr itself.

## Before the first look

Every new run of an agent (started, told to go on, sent back after a review, a conflict or a retry) starts with no
guess, and the first look takes about a minute. The task's line is never a bare "…" meanwhile:

- **It had a sentence before:** that sentence stays, with how old it is before it (`12m ago: Running the check`),
  so it is not read as a new one. The percent starts again at 0.
- **Its first run:** `Just started`, and `Review just started` for a reviewer, who is a new agent every round.
  After three minutes with still no sentence (the Watcher's calls fail) it says `No status yet`.
- **With the Watcher off** no sentence comes, and the line stays empty.
- **How a sentence is written:** the Watcher's prompt shows it a few examples (`Fixing the two failing tests`,
  `Waiting for the PR review on GitHub`), and so do the agents' instructions for reporting. What they write is shown
  exactly as written: the board changes nothing in it.

The list, the task panel, the whole-task page and herdr's own sidebar all say the same words.

## A lighter start

With Claude as the Watcher (the default) the program is started with nothing of yours loaded: no MCP servers, no
plugins, hooks, skills or `CLAUDE.md` (`--safe-mode --strict-mcp-config --disable-slash-commands`), and a two-line
instruction in place of Claude Code's own long one. Logging in works as always. A Claude too old to know these
options is started the plain way from then on. The prompt is smaller too: the last 30 screen lines (3000 characters)
instead of 40 (5000), and 800 characters of the task's description instead of 1500. Codex and pi are started as
before: pi's options for loading less saved nothing when measured.

## Measured

On the 8-core machine this was built on (Claude Code 2.1.296, `claude-haiku-5-5`, load average about 9.5 while
measuring), processor time of the program itself:

| | One call | Six working agents, a minute |
|---|---|---|
| Before: every 15 seconds, a call per agent, up to 4 at once | 1.55 s of processor time, 5.1 s in all | 24 calls: about 37 s of processor time |
| Now: once a minute, one call for all, lighter start | 0.9 s, 2.6 s in all (six agents in one call: 1.1 s, 4.6 s) | 1 call: about 1.1 s of processor time |
| Now, on a machine at twice its capacity | the same | a call every five minutes: about 0.2 s a minute |

That is about a thirtieth of the processor time, and never more than one Watcher program running. For comparison,
the other programs' plain one-shot calls: `codex exec` 1.4 s of processor time, `pi -p` 2.0 s.
