# When an agent could not start

**A task whose agent could not start says why on its row, in plain words, and what to do.** Fix that, then delegate
again (or reply, for an agent that is started again).

| The row says | What to do |
|---|---|
| `Could not start: codex is not installed here` | Install it so that `codex` runs in a terminal here, or choose another program (`worker.agent`). |
| `Could not start: codex is not logged in` | Run `codex` in a terminal here and log in. |
| `Could not start: codex does not know the model it was given`, `Could not start: sol-6.1 can't be used with your ChatGPT sign-in; try gpt-6.1-sol` | Choose a model it has (`herdr-projects models codex` lists the ones your sign-in can use; [models](models.md)), or leave the model out. |
| `Could not start: codex 0.150.0 does not take one of the board's start arguments` | Update the program. |
| `Could not start: codex quit as it started` | Read what it printed, in the task panel, and fix that. |
| `Could not start: codex quit as it started`, and it printed `cannot unmarshal ~/.codex/config.toml` | Its config.toml does not read: `herdr-projects doctor` says which line, or which project's table is in it twice (an older board could add one). Delete the second copy; the board never rewrites the file. |
| `Could not start: codex did not come up within a minute` | Look at its terminal: whatever it shows (an update, a question, an error) is in the way. |
| `Could not start: its start command was cut off: the terminal's shell was still starting` | Delegate again: the board has cleared the half-typed command. |
| `Could not start: its terminal's shell was not ready for the start command` | Wait until the task's terminal shows its prompt, then delegate again. The board already waited and tried 4 times; a shell that is slow to start (its `.zshrc`) causes this. |
| `Could not start: its terminal was not at its shell prompt` | End what runs in the task's terminal, or close it. When the board says it already waited and tried 4 times, the shell may also be very slow to start: wait until it shows its prompt. |
| `Could not start: herdr did not answer in time` | Check that Herdr runs and answers (`herdr workspace list`). |
| `Could not start: its start was cut off, twice` | Delegate again. Something ended the start from outside, most often a script or service that ends all it started with it (a systemd oneshot). While herdr runs on the machine its keeper does too, and `herdr-projects delegate` from a script hands the start to it, so it lives on; a start cut off once is tried again by itself. |

## Where to look further

- **The task panel** has the whole account: the reason, what Herdr said, the last lines of the agent's terminal, and where
  to look further.
- **The terminal stays open**: `Alt+O` on the task goes into it.
- **The task's own log** gets the same account at its end: `runner.log` in the task's folder on the board
  (`~/.herdr-projects/data/tasks/<repo>/<number>/`).
- **A reviewer** that could not start says the same in its review.
- **The Coordinator** says it above the box, and keeps its terminal's last lines in `start.log` in its own folder, since
  its tab is closed.

More problems and where their logs are: [troubleshooting](troubleshooting.md).

## Long rules and long messages

Two limits once made long text fail to start an agent, with no reason given. Both are handled now.

- **Long rules go in a file.** Herdr starts a program by typing its command into the pane's shell. While that shell is
  still starting (its line editor not up yet: a slow `.bashrc` or `.zshrc`), the terminal takes at most 4096 bytes on one
  line, and on macOS only 1024. Codex used to get a task's rules inline, in `-c developer_instructions=…`. With your own
  `worker.instructions`, that went over the limit: the command arrived cut off, the shell waited at its `>` continuation
  prompt, and the start timed out. Now rules longer than 300 bytes go in a file in the task's folder (`rules.md`), and
  Codex is told to read it and follow it for the whole session. Claude Code, pi, Droid and the rest always take their
  rules as a file name, so their start command stays short: Codex alone takes them only as text.
- **A shell not at its prompt yet is waited for.** Herdr 0.9.5 refuses a line longer than the terminal takes before the
  shell's prompt is up ("the terminal is in canonical mode with a 1024-byte limit; wait for the shell prompt and retry"):
  a new task's first start failed that way on macOS. The board now waits for the pane to show its prompt and tries again,
  up to 3 more times within about 15 seconds, for every start (a task's agent, a reviewer, the Coordinator, a
  comparison's picker). It looks for an agent in the pane first, so it never starts a second one.
- **A shell herdr does not see yet is waited for too.** Herdr 0.9.5 also refuses a start in a pane it has just made while
  its shell is still starting (a `.zshrc` that takes a second): "agent target pane … is not an available shell"
  (`agent_pane_busy`). The Coordinator's start after an update failed that way. The board now waits and tries again the
  same way. A pane where a program really runs still fails, after those tries, with "its terminal was not at its shell
  prompt"; one where an agent already runs fails at once.
- **Very long messages go in a file.** A message is one argument of `herdr agent prompt`, and Linux passes at most 128 KiB
  in one argument. A message longer than 100,000 bytes (a task's first prompt, a reply, a message to the Coordinator) is
  written to a file in the board's folder (`messages/`), and a line naming that file is typed in instead. Anything shorter
  is typed in as it is: 4,000 and 100,000 characters went in whole, into Claude Code and Codex alike.

How the limits were found: [checked on Herdr](checked-on-herdr.md).
