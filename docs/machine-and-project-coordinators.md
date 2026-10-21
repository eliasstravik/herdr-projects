# Machine and project coordinators

**Three levels you can talk to, each its own conversation.**

| Level | Talk to it | Named | Lives in Herdr |
|---|---|---|---|
| The [Coordinator](coordinator.md) | the chat, as always | `Coordinator` | a `Coordinator` tab in the home space (`~`) |
| A machine coordinator | `Space` on a machine's heading | after the machine: `vm-str coordinator`, `Mac coordinator` | a tab of that name in that machine's home space |
| A project coordinator | `Space` on a repo's heading | after the repo: `herdr-projects coordinator`, `~ coordinator` for the home folder | a tab of that name in the repo's own space |

The Coordinator stays as it is. The others sit beside it: talking to one never touches the others.

## Talking to one

- **A machine's heading** shows when the board shows every machine (`Alt+M`). `Space` on it opens that machine's
  coordinator in the chat.
- **A repo's heading** shows when the tasks are grouped by repo (`Alt+S`). `Space` on it opens that repo's coordinator.
  The home folder's tasks have one too.
- **A repo with no tasks in view keeps its heading**, folded (`0 tasks`), when it was used in the last 14 days (a task,
  archived ones too) or has a project coordinator, so `Space` still reaches it. Older repos, and repos whose folder is gone, have none.
- The chat names it (`herdr-projects coordinator  idle`), and so does the box: `Send message to herdr-projects
  coordinator`. `Enter` sends, and it decides what to start, as the Coordinator does.
- **It follows the selection**, as a task's agent does: move to another heading and you talk to that one's
  coordinator, move to a task and you talk to its agent. `Space` again goes back to the Coordinator.
- A status group's heading (`to do`, `done`) has none. `Space` there says so.

Your first message starts it. Until then the chat says `not started: your first message starts it`.

## What each one is

- **An ordinary agent in a long-lived chat**, like the Coordinator: the same board commands, its own tab, and settings
  of its own (below). Everything you say is one conversation, kept: if its
  program exits or Herdr restarts, your next message starts it again in the same conversation.
- **A machine coordinator** is about that machine as a whole: its repos and folders, the tasks and agents on it. It runs
  in a folder of its own (`~/.herdr-projects/data/coordinator/<session>/machine/chat`), with its rules in `AGENTS.md`
  there, as the Coordinator's are.
- **A project coordinator** is about one repo. It runs in the repo's own folder, at its main level: your checkout. It
  reads it freely and changes nothing there unless you ask it to; work goes on the board as tasks of that repo. Its tab
  goes in the repo's space (the one Herdr labels after the repo); with none open, one is made there. Since the repo's
  `AGENTS.md` is yours, its rules are a file beside its state
  (`~/.herdr-projects/data/coordinator/<session>/projects/<repo>/rules.md`), passed when it starts, as a working agent's
  are.
  Your own chats in the repo's folder are never taken for its conversation: it keeps the one it has, and moves to
  another (after its own `/clear`) only when Herdr names that one for its pane.
- **Another machine's** coordinators run on that machine. Its board sends what they said with its tasks, and what you
  type goes there. A machine on an older version has none: the chat says to update it there.
- **Herdr's sidebar** names each as the board does, with where it stands and its last message.

## A heading is its coordinator

A machine's heading and a repo's heading stand for their coordinator, as a row stands for its task:

- `Alt+P` shows it beside the list, `Alt+T` whole, in the [task panel](task-panel.md)'s form.
- `Ctrl+E` opens its settings page: its own rows first, under `This coordinator`, then what that heading's page always
  showed (on this machine's heading your own settings, on a repo's your entry for that repo).
- `Tab`, with nothing typed, puts it on [autopilot](autopilot.md) with its saved goal and settings; `Ctrl+X` takes it
  off. While on, the heading reads as a task's row: autopilot's mark, its timing on the right, what it
  does under its name ([autopilot](autopilot.md)).
- Grouped by status, or on one machine, these coordinators have no heading: change the grouping to reach them.

**Each has its own settings**, `coordinator.*` and `autopilot.*`, looked up **this coordinator → you → built in**
([settings](settings.md)). What you set for yourself counts for every coordinator; one set for this coordinator alone
wins over it. `coordinator.instructions` add up: yours, then the coordinator's own.

```
herdr-projects set coordinator.effort high --machine-coordinator
herdr-projects set coordinator.agent codex --project-coordinator herdr-projects
herdr-projects show --project-coordinator herdr-projects
```

## From a shell

Every `coordinator` command takes `--machine` or `--project <repo>`:

```
herdr-projects coordinator --machine                    how it stands
herdr-projects coordinator --machine tell <text>        send it a message
herdr-projects coordinator --project herdr-projects tell <text> [--delegate]
herdr-projects coordinator --project <repo> restart | new | stop
```
