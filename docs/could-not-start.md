# When an agent could not start

A task whose agent could not be started says why on its row, in plain words, and what to do next:

| The row says | What to do |
|---|---|
| `Could not start: codex is not installed here` | Install it so that `codex` runs in a terminal here, or choose another program (`worker.agent`). |
| `Could not start: codex is not logged in` | Run `codex` in a terminal here and log in. |
| `Could not start: codex does not know the model it was given` | Choose a model it has (`worker.agent`, as in `codex/gpt-5.5`), or leave the model out. |
| `Could not start: codex 0.150.0 does not take one of the board's start arguments` | Update the program. |
| `Could not start: codex quit as it started` | Read what it printed, in the task panel, and fix that. |
| `Could not start: codex did not come up within a minute` | Look at its terminal: whatever it shows (an update, a question, an error) is in the way. |
| `Could not start: its start command was cut off: the terminal's shell was still starting` | Delegate again: the board has cleared the half-typed command. |
| `Could not start: its terminal was not at its shell prompt` | End what runs in the task's terminal, or close it. |
| `Could not start: herdr did not answer in time` | Check that herdr runs and answers (`herdr workspace list`). |

Then delegate again (or reply, for an agent that is started again). The task panel has the whole account: the reason,
what herdr said, the last lines of the agent's terminal, and where to look further. The terminal itself stays open
(`enter` on the task goes into it), and the same account is written to the end of the task's own log,
`runner.log` in its folder on the board (`~/.herdr-projects/data/tasks/<repo>/<number>/`). A reviewer that could not
be started says the same in its review; the Coordinator says it above the box, and keeps the terminal's last lines in
`start.log` in its own folder, since its tab is closed.

## Long rules and long messages

Two limits once made long text fail to start an agent, with no reason given:

- **A start command is typed into a terminal.** herdr starts a program by typing its command into the pane's shell,
  and while that shell is still starting (its line editor not up yet: a slow `.bashrc` or `.zshrc`) the terminal
  takes at most 4096 bytes on one line. Codex took a task's rules inline, in `-c developer_instructions=…`: with
  your own `worker.instructions` that was over 4096 bytes, the command arrived cut off, the shell waited at its
  `>` continuation prompt and the start timed out. Rules longer than 2000 bytes now go in a file in the task's folder
  (`rules.md`, as for Claude Code and pi), and Codex is told to read it and follow it for the whole session.
- **A message is one argument of `herdr agent prompt`**, and Linux passes at most 128 KiB in one argument. A message
  longer than 100,000 bytes (a task's first prompt, a reply, a message to the Coordinator) is written to a file in the
  board's folder (`messages/`), and a line naming that file is typed in instead. Anything shorter is typed in as it
  is: 4,000 or 100,000 characters went in whole, into Claude Code and Codex alike.

How the limits were found: [checked on herdr](checked-on-herdr.md).
