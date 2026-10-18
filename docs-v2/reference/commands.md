# Commands

This page lists the commands of `herdr-projects`, what each command answers, and how a command treats its flags.

## The commands

`herdr-projects help` lists the commands:

| Command | More information |
|---|---|
| `add`, `list`, `report`, `describe`, `delegate` | |
| `after` | [One task after another](../explanation/after.md) |
| `tell`, `stop`, `approve`, `review`, `retry`, `fix`, `finish`, `details` | |
| `pin`, `archive`, `restore`, `clear` | |
| `set`, `unset`, `show` | [Settings](settings.md#commands) |
| `automations` | [Automations](automations.md) |
| `machines` | [Other machines](../explanation/other-machines.md) |
| `install`, `update` | [Install and update](../how-to/install-and-update.md) |
| `remove-extras` | [Roles](roles.md#the-how-to-in-every-chat) |

These commands are also on other pages:

- `herdr-projects models` and `herdr-projects models <program>` list models and effort levels ([models](models.md)).
- `herdr-projects coordinator new --agent …` starts a new Coordinator for one use ([roles](roles.md#the-coordinator)).
- `herdr-projects details <id>` prints the reviews of a task, the same list as `reviews`
  ([the task panel](task-panel.md#the-reviews)).
- `report` takes `--progress` and `--eta` for your own tasks ([roles](roles.md#the-watcher)).

## What a command answers

A command answers with one line that starts with the id of the task. It uses the same few words everywhere. The
messages of the board and the closing line of the Coordinator use them too.

| Command | Answers |
|---|---|
| `add` | `proj/12: added`, or `proj/12: added, an agent is on it` with `--delegate` |
| `describe`, a new title, `report` | `updated`, with what changed: `updated (description)`, `updated (title)`, `updated (done)` |
| `delegate` | `started` |
| `clear` | `removed` |
| `archive`, `restore` | `archived`, `restored` |
| `approve` | `approved` |

`approve` refuses work whose last review failed. It tells what the review found. `approve <id> --anyway` merges the
work all the same. Use `--anyway` only when the human said so in clear words.

## Flags

Each command takes only its own flags, and also `--json` and `--help`.

- `add`, `delegate` and `automations` also take any work setting by its name.
- Settings that you give to `add` belong to the task from the start, with `--delegate` or without it.
- `--research` and `--no-research` on `add` and `delegate` are a short form of `research.only`
  ([settings](settings.md#the-list)).
- The list of flags is in `src/cli/flags.rs`.

The board refuses a flag that a command does not know. The command stops before it does anything, and it names the
nearest correct flag:

```
$ herdr-projects list --archvied
list does not know --archvied. Did you mean --archived?
```

Note: In earlier versions, the command ignored a wrong flag. Then the command did a different thing from what you
asked.

## Related pages

- [Settings](settings.md)
- [Roles](roles.md)
- [Models](models.md)
- [Change a setting](../how-to/change-a-setting.md)
- [Gate rules](gate-rules.md)
