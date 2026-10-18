# How the board uses agent programs

This page tells how the board starts and follows an agent program, and why it refuses some programs for some roles.
For the list of programs and the table of their flags, read [agent programs](../reference/agent-programs.md).

## Why you start a program one time yourself

Before a program does its first task, you must start it one time in a terminal, in a folder of a repo. Log in, do its
first setup, and trust the folder if it asks.

Herdr reads a login page as a program at rest. Thus the board types the task into a logged-out program as if the
program is ready. Cursor, Devin, Grok, agy and Letta all came up at a login page or a setup page when the project
tried them.

A folder-trust dialog in a new worktree is a dialog like all others: the task waits for you (`needs you`). The board
tells Claude Code and Codex before they start to trust the folder ([how work gets done](how-work-gets-done.md)).

If a start fails after that, the task tells why ([when an agent could not start](could-not-start.md)).

## How the board starts a program

The board starts a program as Herdr starts it (`herdr agent start --kind <name>`), with the arguments of the board after
it. Herdr finds the program in its pane and tells whether it works or waits.

The table of programs comes from the code (`crates/core/src/agents/kinds/`). Thus it always agrees with what the board
does.

## Helpers (subagents)

An agent on a task uses the helpers of its program as it does outside the board. Examples are the subagents of Claude
Code, the spawned agents of Codex, the tasks of opencode, and skills that use them.

The arguments of the board limit no tool. The rules of the board say only that the agent must not give the work to
another task or another agent on the board. The rules name helpers as allowed, also when the task or a skill asks for
them.

The reason: an earlier rule said only "never hand it to anyone else". Claude and Codex read that as "no helpers", and
refused helpers also when the task asked for them (HP-53).

## What the board knows about each program

The board reads the conversations that Claude Code, Codex and pi save. Thus it knows what they said, what went wrong with
the provider, and how many tokens they used ([how work gets done](how-work-gets-done.md)).

The board reads the conversation of no other program. For those programs, it gets the end of a turn from Herdr, and
what the agent said from its closing note. The full list is in
[agent programs](../reference/agent-programs.md#what-the-other-programs-do-not-get).

Because the board does not read their conversations, these programs have three limits:

- The board cannot count their tokens.
- The board cannot tell a passing provider error from a lasting one. Thus the board does not wait and try again, and
  the fallback agent does not take over for that error. A logged-out screen still causes a hand-over.
- The board cannot follow the helpers that the agent starts.

## Why the board refuses some programs

The board refuses a program for a role that it cannot do well. It refuses it when you set it, not when a task starts.

- **MastraCode works on no task.** When it starts, the board can tell it neither its model nor to work without asking.
  MastraCode takes these only through environment variables, and the start of Herdr cannot set them. Also, MastraCode
  opens the last conversation of its folder again, so a reviewer joins the conversation of the worker. MastraCode can
  be the Watcher.
- **The Watcher needs a headless one-shot**: a command that answers one time and ends. Every program on the list has
  one. The board refuses a program without one.
- **Droid takes no model when it starts for work.** Choose its model in the settings of Droid. Its one-shot takes a
  model, so `watcher.agent droid/<model>` works.
- **`pi` never asks before it acts.** Thus the board refuses `access ask` for it.

## Why some problems show only at start

The board can find two problems only when the program starts:

- **`omp` and MastraCode need the Herdr integration.** Without it, Herdr cannot see when they work. The board does not
  install the integration into the settings of your agent itself. It tells you to install it.
- **Herdr cannot see some programs.** Kilo Code and Qoder, when you install them with npm, run behind a small node
  launcher. The launcher starts the real program as its child. Herdr 0.9.3 does not find that child in the pane. The
  standalone build does not have this problem: the project tried it, and Herdr found the own binary of Kilo at once.

## Programs that Herdr starts without an integration

Herdr also starts Gemini CLI, Cline, Kiro, Amp, Maki and Muse. Herdr has no integration for them, and calls them "less
tested". The board does not take them. To support one, a contributor adds one more entry to the table of programs
([the code of the agent programs](../contributing/agents-code.md)).

## Related pages

- [Agent programs](../reference/agent-programs.md)
- [How work gets done](how-work-gets-done.md)
- [When an agent could not start](could-not-start.md)
- [The fallback agent](fallback.md)
- [A turn cut off by the provider](provider-errors.md)
- [The code of the agent programs](../contributing/agents-code.md)
