# How the roles work

This page tells why the board uses four roles, why the guess of the Watcher can go down, and why the agents that the
board starts do not change the board. The facts are in [roles](../reference/roles.md).

## Four roles, four jobs

Each role has one job. The working agent does the work. The reviewer attacks the finished work, to find what is wrong.
The Watcher is a small model. It writes the "doing now" line and guesses how much of the work is done. The Coordinator
is the agent that you talk to. It changes your words into tasks.

Each role has its own settings. Thus the Coordinator stays Claude Code on its default model and effort, whatever you
set for the working agent. Only the `coordinator.` settings, or `coordinator new --agent …` for one use, change it.

## Why the guess of the Watcher can go down

The Watcher does not count up only. At each look, it sees its last guess and the last lines on the screen of the agent.
It keeps its last guess unless the screen gives a reason to change it. A failing test, a merge conflict or review
findings mean more work. Thus the guess can go down.

The minutes left come from the time spent and the percent. Below 5%, the percent is too small to give a useful number.
Thus the board shows no minutes then.

## Why agents can act on the board, but do not

An agent that the board started can run every command, as you can. The board does not block these commands. A human can
ask an agent to use them. For example, a task can say "add a task for each failing page and delegate it".

But the rules of the agent do not name these commands. They tell the agent to do nothing to the board beyond its own
task, unless a human told it to explicitly. Thus an agent does not make tasks or change other tasks without your
instruction.

Work that the agent only finds goes in an **Also noticed** list at the end of its closing note or report. Thus you see
it, and you decide what to do with it. A `preexisting` finding of the reviewer is the same kind of note.

## Why every chat knows the board

The plugin puts a how-to in the skills folder of each installed agent program. Thus any usual chat can add a task to
the board without instructions. The board does this only for the programs whose skills folder it knows.

## Related pages

- [Roles](../reference/roles.md)
- [The Watcher](watcher.md)
- [The Coordinator](coordinator.md)
- [The gate](gate.md)
- [Commands](../reference/commands.md)
