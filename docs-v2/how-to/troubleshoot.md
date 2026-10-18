# Troubleshoot the board

This page tells what to do for the usual problems. Most problems show on the row of the task, with the next step.

## Start here

1. Select the task.
2. Press `Alt+P` to open the task panel. Or press `Alt+T` to open it in full screen. The task panel has the full
   account.
3. If the cause is not clear, run `herdr-projects doctor`. It examines all the usual problems in approximately one
   second, and tells what to do for each problem
   ([the health check and the logs](../reference/troubleshooting-checks-and-logs.md)).
4. Find the symptom on this page, or on one of these pages:
   - [Fix an agent that could not start](could-not-start-fix.md)
   - [Fix a task that seems stuck](troubleshooting-stuck-task.md)
   - [Fix a task that is behind main](troubleshooting-behind-main.md)

## An agent could not start

The row says `Could not start: …` with the reason, for example `codex is not logged in` or
`sol-6.1 can't be used with your ChatGPT sign-in`.

1. Do what the row says. Each reason and its fix is in
   [fix an agent that could not start](could-not-start-fix.md).
2. Delegate the task again.

**Note:** For a program that you have not used with the board before, start it one time yourself, in a terminal, in a
folder of a repo. Log in, do its first setup, and trust the folder if it asks
([before the first task of a program](could-not-start-fix.md#before-the-first-task-of-a-program)).

## A model name is refused

The board refuses a wrong name when you set it. It gives the names that the program takes, and the nearest one:

```
$ herdr-projects set worker.agent codex/sol-6.1
worker.agent: codex has no model "sol-6.1" for your ChatGPT sign-in (pro plan). Did you mean gpt-6.1-sol? It takes
gpt-6.1-sol, gpt-6-astra, gpt-6-sol, … (codex's own list for that sign-in). Write it after the program: codex/gpt-6.1-sol.
```

1. To see the names that a program takes, run `herdr-projects models codex`. For Codex, this is what your own sign-in
   can use. A ChatGPT sign-in and an API key take different names.
2. Write the model after the program, for example `codex/gpt-6.1-sol`.

## The board says "its list cannot be read"

The program is installed, but its list command failed. For example, the program is not logged in, or it is offline.

1. Run that list command in a terminal to see why.
2. Or remove the model from the setting.

## A new Claude model is refused

The table of Claude names in the board is older than the model.

1. Use the short name, for example `claude/opus`. The short name always means the newest model of its kind.
2. To get the new names, update the board: `herdr-projects update`.

## An effort is refused

Each program has its own levels.

1. Read the levels in the refusal.
2. Set one of those levels.

## "can't be used with your ChatGPT sign-in" while the agent works

Codex refused the model at the first turn of the agent
(`The 'sol-6.1' model is not supported when using Codex with a ChatGPT account`). The board catches this.

If the row says **`…: started again on codex's own default model`**, the board had given that model to Codex. Codex
now works on its own default model.

1. Do nothing.
2. If you want a different model, run `herdr-projects models codex`. It lists the models that your sign-in can use.

If the row says **`Agent stopped: … can't be used with your ChatGPT sign-in; try gpt-6.1-sol`**, Codex also refused its
default model. Or the board gave it no model: then the name comes from the `model` of Codex in `~/.codex/config.toml`.

1. Choose a model for the task: `herdr-projects set worker.agent codex/gpt-6.1-sol --task <id>`. Or correct that line in
   `~/.codex/config.toml`.
2. Press `Ctrl+L` (retry). The agent starts again on that model.

For more, read [models](../reference/models.md) and [choose a model and an effort](choose-a-model-and-effort.md).

## A done task stays in the Herdr sidebar

The space of a merged or finished task closes by itself (`workspace.cleanup`, on by default). When a space stays, its
line in the task panel ends in `Kept:` and tells why.

If it says **`(uncommitted changes)`**, its checkout has files that nobody committed.

1. Look at the files.
2. Reply to the task, or remove the worktree yourself.

If it says **`(unmerged work on <branch>)`**, its branch holds commits that nobody merged.

1. Merge the commits, or reply to the task to continue with them.

If it says **`(its space is in use again)`**, there is a turn in the terminal of its agent, a command that still runs in
one of its panes, or an agent that Herdr cannot place.

1. Do nothing. The board tries again approximately each half minute, and closes the space when it is free.

If it says **`(its herdr session cannot be reached)`**, the Herdr session that it ran in does not run.

1. Start that session. The board tries again.

If nothing says `Kept:` and the space still stays:

1. Run `herdr-projects show <id>`. It tells whether somebody turned off `workspace.cleanup`, and where.
2. To turn it on again, run `herdr-projects set workspace.cleanup true`.

To close a space at once:

1. Archive the task (`Ctrl+A`). What the board keeps on disk stays, and the task tells where.

**Note:** An agent with MCP servers stayed `in use` for all time in earlier versions: its servers counted as a command
that still ran. This is fixed. The next pass closes those spaces by itself.

## `herdr-projects` is not found

1. Open the board one time from Herdr (the action "Open the work board" of the plugin).
2. Add `~/.herdr-projects/data/bin` to your PATH ([put the command on your PATH](install-and-update.md#put-the-command-on-your-path)).

## The board key does nothing

1. If a popup of another plugin is open, close it first.
2. If the key is not bound, bind it. The board offers the key on its first screen
   ([do the first-run setup](install-and-update.md#do-the-first-run-setup)).

## The board looks cut off or half empty

Another Herdr window on the same session (a second device, a browser tab) has the size
([the size of the board](../explanation/board-size.md)).

1. Type a key in this window.

## Herdr refuses the plugin

Your running Herdr server must be 0.9.3 or newer.

1. Update Herdr.
2. Restart the Herdr server.

## The tasks of another machine do not show

1. Run `herdr-projects machines`.
2. Read [when a machine does not answer](../explanation/other-machines.md#when-a-machine-does-not-answer).

## Related pages

- [The health check and the logs](../reference/troubleshooting-checks-and-logs.md)
- [Fix an agent that could not start](could-not-start-fix.md)
- [Fix a task that seems stuck](troubleshooting-stuck-task.md)
- [Fix a task that is behind main](troubleshooting-behind-main.md)
- [A turn cut off by the provider](../explanation/provider-errors.md)
- [The task panel](../reference/task-panel.md)
- [Settings](../reference/settings.md)
