# Roles

This page lists the four roles of the agents that the board starts, and the facts about each role. To learn why the
roles work as they do, read [how the roles work](../explanation/roles.md).

## The four roles

| Role | What it does |
|---|---|
| **Agent** (the working agent) | Does the work. |
| **Reviewer** | Attacks the finished work, to find what is wrong. |
| **Watcher** | A small model that writes the "doing now" line and guesses how much of the work is done. |
| **Coordinator** | The agent that you talk to in the chat. It changes what you type into tasks. |

You choose the program and the model of each role with its settings: `worker.`, `reviewer.`, `coordinator.` and
`watcher.`. The Agent, the Reviewer and the Coordinator also have an effort setting. The Watcher has no effort setting
([settings](settings.md), [models](models.md)).

## The Watcher

The Watcher looks at each working agent approximately one time each minute. If the screen did not change, it does not
look. At each look, the board gives it:

- the task and its description
- the last lines on the screen of the agent
- the time that the work has run
- its own last guess

The Watcher guesses one number: how much of the work is done, in percent. It keeps its last guess, unless the screen
gives a reason to change it. The guess can go down: a failing test, a merge conflict or review findings mean more work.
More information: [the Watcher](../explanation/watcher.md).

The board changes the percent into minutes left:

```
time spent × (100 − percent) ÷ percent        40 minutes in at 80% → 10 minutes left
```

- The time spent starts when the agent started on the last thing that you asked it to do. While the reviewer works,
  the time spent starts at the start of that review round.
- The board calculates the minutes again each second. Thus they continue to change between two looks.
- Below 5%, there is not sufficient data. The board shows no minutes.
- Your own tasks are different. They show the `--progress` and `--eta` that you give to `report`, as you give them.

## The Coordinator

The Coordinator is one chat that stays open for a long time. It changes what you type into board commands. It can also
do all the other work that an agent chat can do.

- It is Claude Code on its default model and effort, unless you set a different one. The setting of the Agent has no
  effect on it.
- Change it with these settings:
  - `coordinator.agent`: `claude`, `codex`, `pi` or a different program that Herdr starts, with a model after a slash
    ([agent programs](agent-programs.md)).
  - `coordinator.effort`
  - `coordinator.fast`
  - `coordinator.access`
  - `coordinator.instructions`
- To pick a Coordinator for one use only, type `coordinator new --agent …`.
- A changed default applies to the next new Coordinator. The chat tells you so.
- In these conditions, the chat shows "New task", and the board adds what you type as a task, word for word:
  - `coordinator.agent` is `none`.
  - The program is not installed.
  - Herdr does not run.

## The how-to in every chat

The plugin puts a small how-to in the skills folder of each agent program that is installed. The how-to is a folder
named `herdr-projects` that holds `SKILL.md`. The folders are:

- `~/.claude/skills`
- `~/.codex/skills`
- `~/.pi/agent/skills`

Thus any usual chat can "add this to the board" without instructions.

- The how-to gives the command by its full path. Thus it works when `herdr-projects` is not on your PATH.
- The other programs of Herdr do not get the how-to, because the board does not know their skills folders.
- `herdr-projects remove-extras` removes exactly what the plugin wrote. The plugin does not write it again. The plugin
  does not touch a folder of that name that it did not write.

## Agents that the board started

An agent or a reviewer that the board started on a task can run every command, as the Coordinator or an agent in your
own shell can. Examples: `add` (also with `--delegate`), `delegate`, `tell`, `stop`, `approve`, `archive`, `restore`,
`clear`, `set`, `unset` and `describe`. The board does not refuse or change any of these commands.

But the rules of these agents name none of these commands. The rules say: **do nothing to the board beyond your own
task (reporting your own state) unless a human explicitly told you to.**

- An explicit instruction in the description of the task, or in a message from you, counts. For example, if a task
  says "add a task for each failing page and delegate it", its agent does that.
- The agent never makes a task from work that it only finds. It writes that work in a short **Also noticed** list at
  the end of its closing note or report. It does nothing more with it.
- A `preexisting` finding of a reviewer is the same thing in a review: a note ([the gate](../explanation/gate.md)).
- For its own task, the rules tell the agent to report itself `blocked` and nothing else. The finish line marks its work
  done. But if the agent reports `done`, `idle` or `error`, the board accepts it, as from any other person.

## Related pages

- [How the roles work](../explanation/roles.md)
- [Commands](commands.md)
- [Models](models.md)
- [Settings](settings.md)
- [The Coordinator](../explanation/coordinator.md)
- [The Watcher](../explanation/watcher.md)
- [Agent programs](agent-programs.md)
