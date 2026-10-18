# Fix an agent that could not start

This page tells what to do for each `Could not start: …` message on the row of a task. For why a start can fail, read
[when an agent could not start](../explanation/could-not-start.md).

After each fix, delegate the task again. For an agent that the board starts again, reply to it.

## Before the first task of a program

Do these steps one time for each new program. They prevent most failed starts
([why](../explanation/agent-programs.md#why-you-start-a-program-one-time-yourself)).

1. Open a terminal in a folder of a repo.
2. Start the program.
3. Log in.
4. Do the first setup of the program.
5. If the program asks you to trust the folder, trust it.

## `Could not start: codex is not installed here`

1. Install the program, so that `codex` runs in a terminal on this machine.
2. Or choose another program with `worker.agent`.

## `Could not start: codex is not logged in`

1. Run `codex` in a terminal on this machine.
2. Log in.

## `Could not start: codex does not know the model it was given`

The row can also say, for example,
`Could not start: sol-6.1 can't be used with your ChatGPT sign-in; try gpt-6.1-sol`.

1. Run `herdr-projects models codex`. It lists the models that your sign-in can use ([models](../reference/models.md)).
2. Choose a model from that list.
3. Or remove the model from the setting.

## `Could not start: codex 0.150.0 does not take one of the board's start arguments`

1. Update the program.

## `Could not start: codex quit as it started`

1. Open the task panel.
2. Read what the program printed.
3. Fix that problem.

## `Could not start: codex did not come up within a minute`

1. Look at the terminal of the agent.
2. Remove the cause that it shows: an update, a question, or an error.

## `Could not start: its start command was cut off: the terminal's shell was still starting`

The board already removed the half-typed command.

1. Delegate the task again.

## `Could not start: its terminal's shell was not ready for the start command`

The board already waited and tried 4 times. A shell that is slow to start (because of its `.zshrc`) causes this.

1. Wait until the terminal of the task shows its prompt.
2. Delegate the task again.

## `Could not start: its terminal was not at its shell prompt`

1. End the command that runs in the terminal of the task, or close the terminal.

## `Could not start: herdr did not answer in time`

1. Run `herdr workspace list`.
2. If it does not answer, make sure that Herdr runs.

## `herdr did not see kilo come up in its pane …`

Herdr 0.9.3 cannot see Kilo Code or Qoder when you install them with npm.

1. Install the standalone build of the program.

## `omp needs herdr's own integration, or herdr cannot see when it works: …`

1. Run `herdr integration install omp`.
2. Start the task again.

## Look further

These places keep the full account of a failed start:

- **The task panel** has the full account: the reason, what Herdr said, the last lines of the terminal of the agent, and
  where to look further.
- **The terminal stays open.** Press `Enter` on the task to go into it.
- **The log of the task** gets the same account at its end: `runner.log` in the folder of the task on the board
  (`~/.herdr-projects/data/tasks/<repo>/<number>/`).
- **A reviewer** that could not start tells the same in its review.
- **The Coordinator** tells it above the box. Its tab is closed, so it keeps the last lines of its terminal in
  `start.log` in its own folder.

For more problems and their logs, read [troubleshoot the board](troubleshoot.md) and
[the health check and the logs](../reference/troubleshooting-checks-and-logs.md).

## Related pages

- [When an agent could not start](../explanation/could-not-start.md)
- [Troubleshoot the board](troubleshoot.md)
- [Agent programs](../reference/agent-programs.md)
- [Models](../reference/models.md)
