# Roles and commands

## Four roles

| Role | What it does |
|---|---|
| **Agent** | does the work |
| **Reviewer** | attacks the finished work, looking for what is wrong |
| **Watcher** | a small model that writes the "doing now" line and guesses how far along the work is |
| **Coordinator** | the agent you talk to in the chat; it turns what you type into tasks |

### The Watcher's guess

About once a minute (skipping a look when the screen has not changed), the Watcher is shown the task, its description,
the last lines on the agent's screen, how long the work has run, and its own last guess. It guesses one number: how
far along the work is, in percent. It keeps its last guess unless the screen gives a reason to change it. The guess can
go down: a failing test, a merge conflict or review findings mean more work. More: [the Watcher](watcher.md).

The board turns that percent into minutes left:

```
time spent × (100 − percent) ÷ percent        40 minutes in at 80% → 10 minutes left
```

- Time spent counts from when the agent began on what you last asked of it. While the Reviewer works, it counts from
  the start of that review round.
- The minutes are worked out again every second, so they keep moving between looks.
- Under 5% there is too little to go on, and no minutes are shown.
- Your own tasks are different: they show the `--progress` and `--eta` you give `report`, as given.

### The Coordinator

One long-lived chat that turns what you type into board commands, and can do anything else an agent chat can.

- It is Claude Code on its default model and effort unless you say otherwise, whatever the Agent is set to.
- Change it with `coordinator.agent` (`claude`, `codex`, `pi` or another program Herdr starts, with a model after a
  slash: [agents](agents.md)), `coordinator.effort`, `coordinator.fast`, `coordinator.access` and
  `coordinator.instructions`. Or pick one for a single use with `coordinator new --agent …`.
- A changed default applies to the next new Coordinator. The chat says so.
- With `coordinator.agent none`, a program that is not installed, or no Herdr running, the chat says "New task" and
  what you type is added word for word.

## Every chat knows the board

The plugin puts a small how-to in each installed agent program's own skills folder: a folder named `herdr-projects`
holding `SKILL.md`, in `~/.claude/skills`, `~/.codex/skills` and `~/.pi/agent/skills`. So any ordinary chat can "add
this to the board" without being told how.

- It gives the command by its full path, so it works without `herdr-projects` on your PATH.
- Herdr's other programs do not get it: the board does not know their skills folders.
- `herdr-projects remove-extras` removes exactly what was written, and it is not written back. A folder of that name
  the plugin did not write is left alone.

## Commands

`add`, `list`, `report`, `describe`, `delegate`, `after` ([one task after another](after.md)), `tell`, `stop`,
`approve`, `review`, `retry`, `fix`, `finish`, `details`, `pin`, `archive`, `restore`, `clear`, `set`, `unset`, `show`,
`automations` ([automations](automations.md)), `machines` ([other machines](other-machines.md)), `install`, `update`
([installing](install.md)), `remove-extras`.

`herdr-projects help` lists them.

### What a command answers

One line that starts with the task's id, in the same few words everywhere (the board's own messages and the
Coordinator's closing line use them too):

| Command | Answers |
|---|---|
| `add` | `proj/12: added`, or `proj/12: added, an agent is on it` with `--delegate` |
| `describe`, a new title, `report` | `updated`, with what changed: `updated (description)`, `updated (title)`, `updated (done)` |
| `delegate` | `started` |
| `clear` | `removed` |
| `archive`, `restore` | `archived`, `restored` |
| `archive --done [--repo NAME]` | `Archived 50 done tasks of mstars.`, and what it left out: `Left out: 1 done but pinned, 2 working.` |
| `archive --repo NAME`, `restore --repo NAME` (`--on MACHINE`: another machine's) | `Archived mstars and its 6 tasks.`, `Restored mstars and its 6 tasks.`, and the tasks it stopped first ([archiving a project](board.md#archiving-a-project)) |
| `approve` | `approved` |
| `stop` | `stopped`, and what it ended beside the agent: `proj/12: stopped. Ended what it started: bun …/check (pid 812).` |

`stop` interrupts the agent and keeps it, so a reply goes on, and ends what it started for the task: every process
in its terminal below its shell but the agent and its helpers (an MCP server wired to it), the whole tree, and your
processes that run from the task's worktree or temporary folder. Never your own checkout, another task's worktree, or
what runs in the task's other panes (yours, its reviewer's). One started for the task that works elsewhere now is named
for you to end, not ended: `Still running, end it yourself: node …/x.js (pid 9), in /srv/app.` So is what runs from its
folders when herdr does not say what runs in its panes: then the agent and your shells there cannot be told apart.

`approve` refuses work whose last review failed, and says what the review found. `approve <id> --anyway` merges it all
the same. It is for when the human said so in so many words.

### A flag a command does not know is refused

Each command takes only its own flags, plus `--json` and `--help`. `add`, `delegate` and `automations` also take any
work setting by its name. Anything else stops the command before it does anything, and names the nearest right flag:

```
$ herdr-projects list --archvied
list does not know --archvied. Did you mean --archived?
```

(A mistyped flag used to be ignored, so the command did something other than what you asked.)

Settings given to `add` belong to the task from the start, with `--delegate` or without. The list of flags is
`src/cli/flags.rs`.

## Agents the board started: able, but not nudged

An agent or reviewer the board started on a task can run every command above, like the Coordinator or an agent in
your own shell: `add` (also with `--delegate`), `delegate`, `tell`, `stop`, `approve`, `archive`, `restore`, `clear`,
`set`, `unset`, `describe` and the rest. Nothing is refused or changed.

But their rules name none of these commands, and say plainly: **do nothing to the board beyond your own task
(reporting your own state) unless a human explicitly told you to.**

- An explicit instruction in the task's own description, or in a message from you, counts. So a task that says "add a
  task for each failing page and delegate it" has its agent do that.
- It never writes into the board's own folder (`~/.herdr-projects`) itself, by hand or with a program of its own,
  beyond the files its rules name (its closing note, a reviewer's verdict and evidence): the board changes only
  through its commands.
- Work it merely comes across is never made a task. It goes in a short **Also noticed** list at the end of its closing
  note or report, and nothing else is done with it. A reviewer's `preexisting` finding is the same thing in a review: a
  note ([the gate](gate.md)).
- For its own task, its rules tell it to report itself `blocked` and nothing else (the finish line marks its work
  done). But a `report` of `done`, `idle` or `error` from it is taken like anyone's.
