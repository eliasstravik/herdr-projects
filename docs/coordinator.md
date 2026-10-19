# The Coordinator

In [Mission Control](mission-control.md), the Coordinator is the visible orchestrator pane to the right of the project sidebar and task input. It follows the global CLI priority and reviews worker results in this same conversation. The separate-tab descriptions below concern the classic board.

**The Coordinator is the agent you talk to in the board's chat.** Type what needs doing and it turns it into tasks.
It is one long-lived agent chat, not a call per message: everything you type goes into the same conversation, so
"delegate that one too" works, and it can ask you something back.

Each machine and each repo has a coordinator of its own too, beside this one: `Space` on its heading
([machine and project coordinators](machine-and-project-coordinators.md)).

- `Enter` sends your message.
- `Tab` sends it and **starts an agent on every task it adds** from that message.
- `Alt+C` shows and hides its latest message on the board.

From a shell:

```
herdr-projects coordinator                     how it stands
herdr-projects coordinator tell <text>         send it a message
herdr-projects coordinator tell <text> --delegate   the same as Tab
herdr-projects coordinator restart             start it again, same conversation
herdr-projects coordinator new                 a fresh conversation
herdr-projects coordinator new [--agent KIND[/MODEL]] [--effort E]    a different one, this once
herdr-projects coordinator stop
```

Restart and new are commands, not board keys. You can also just ask the Coordinator to do them. `new --agent …` changes
only the next one: your default stays as it is.

## Your message, and where you were

Your words reach it exactly as typed. Which repo you were in and which task you had open are kept beside each message,
and the Coordinator looks them up itself (`herdr-projects coordinator where` lists the last few messages, each with its
own).

A board opened from the Coordinator's own pane stays in the repo it was last in: that pane's folder is nobody's repo.

## `Tab`: "and start it right away"

A message sent with `Tab` has every task the Coordinator adds from it delegated at once, however many, with nothing
more to confirm.

- This does not depend on the Coordinator remembering. The message carries one line under your words saying it was
  sent with `Tab` (the board's chat shows your words alone). And the board does it too: a task the Coordinator adds
  during the turn of a `Tab` message is delegated by `add` itself.
- A message that adds nothing (a question, a status check, a word about an existing task) delegates nothing extra.
- A message sent with `Enter` adds its tasks, and delegates them only when you asked for that.

## What it says it did

When it acted on the board, its message ends with one short line naming the ids in plain words: added, updated (a
title, a description, settings), removed, started an agent, approved, archived, restored. For example: "Added proj/12
and started an agent on it". The commands answer in the same words ([roles and commands](roles-and-commands.md)).

## Going into it

On the board you see only what it said and what you said, never its program's screen ([the board](board.md)). For the
whole conversation, its dialogs, slash commands and menus, **go into it** in Herdr:

- It is an ordinary agent in the `Coordinator` tab of your home space.
- The plugin action `herdr-projects.coordinator` (bind a key to it) goes straight to its pane from anywhere in Herdr,
  and starts it first if it is not running.
- Get back the way you do from any agent: your board key, or `herdr-projects`.

## What it is

- An ordinary Herdr agent, in a tab named `Coordinator`, in your home space (`~`).
- It runs in a folder of its own inside the board's data folder. `HERDR_PROJECTS_COORDINATOR_DIR` names another; use
  that folder for nothing else, since the board writes the Coordinator's rules there.
- It starts with the same arguments as any working agent: all its tools, nothing held back.
- Your first message starts it, and it stays. If its program exited or Herdr restarted, your next message starts it
  again in the same conversation.
- **Closing the board does not stop it**, and the board says so as you close it. One Coordinator serves every board of
  the machine (a tab, the popup, the web board) and `coordinator tell`, and a turn of it may still be at work.
  `herdr-projects coordinator stop` closes its tab; your next message starts it again, in the same conversation.
- The agent's own new-chat command (`/clear`, `/new`) is followed: the chat and a restart go with the conversation it
  is in now.
- Its program, model and effort are settings (`coordinator.agent`, …): [roles and commands](roles-and-commands.md).

## Its standing rules are a file

The rules (what the Coordinator is, how it uses the board) are written as `AGENTS.md` in the folder it starts in
(`~/.herdr-projects/data/coordinator/<session>/chat`, see [the home folder](home-folder.md)). The program reads that file
by itself at the start of every conversation. So it still knows its rules after its own `/new` or `/clear`, after
`coordinator restart` and `coordinator new`, after `coordinator new --agent <another kind>` (the next program reads the
same file), and when you start the program in that folder by hand.

| Kind | Reads | So the board writes |
|---|---|---|
| Claude Code | `CLAUDE.md` (2.1.296 also reads an `AGENTS.md` beside it) | `AGENTS.md`, and a `CLAUDE.md` that only passes on to it (`@AGENTS.md`): the rules arrive once |
| Codex | `AGENTS.md` | `AGENTS.md` |
| pi | `AGENTS.md` | `AGENTS.md` |
| any other kind | not known | `AGENTS.md`, and one line before its first message that points at the file |

- **Kept up to date.** The files are written again at every start and before every message. So a new version's rules
  are in place under a running Coordinator, which reads them at its next new chat or restart.
- **Your own notes are kept.** The board owns only the block between its two marker lines. What you write above or
  below it in `AGENTS.md` or `CLAUDE.md` stays, and the Coordinator reads it too.
- **One way per kind, never two.** The Coordinator gets no launch flag for its rules any more (it used to: Claude's and
  pi's appended system prompt, Codex's `developer_instructions`). A file works the same for every kind and outlives the
  program. A flag is gone with the process, must be passed again by whoever starts the program, and Codex's could not
  carry this much text on one launch line. With the file in place, a flag would say the same thing twice. Only a kind
  the board does not know, which may or may not read `AGENTS.md`, also gets the pointing line.
- **Working agents and reviewers are different.** They work in your repo, whose `AGENTS.md` is yours, so their rules
  still ride on their launch.

## Where it lives in Herdr

It has no space of its own in the sidebar. It is a tab named `Coordinator` in the home space, the one Herdr labels
`~`: a whole tab, never a pane inside one of yours. In the sidebar's list of agents it is named `Coordinator`, with its
state as shown above the board's box (`Coordinator · idle`) and its latest message under it
([Herdr's sidebar](sidebar.md)).

- **Found or started from anywhere.** A message from the board in a pane, from the board in Herdr's popup, or from a
  shell (`coordinator tell`) goes to the running Coordinator. If none runs (its program exited, its tab or the home
  space was closed, Herdr restarted), the message starts it again in the same conversation, in a `Coordinator` tab of
  the home space. There is never a second one, and no space is made for it.
- **No home space on this machine?** One is made (at your home folder, as Herdr would), then the tab is added.
- **Nothing you look at changes.** The tab, and a home space that had to be made, open without focus: you stay where
  you are, and the board and every other tab keep their size and layout.
- **Which space is home.** The one labelled `~`. Herdr labels a space after the folder its first tab is in, so the
  label changes when you `cd` elsewhere in that tab. The board remembers the space it last put the tab in, and uses
  that when no space is labelled `~`. A task's space is never taken for it.
- **Rename its tab if you like.** The board finds the Coordinator by its pane, as it does a task's agent and reviewers
  ([a task's tabs](tabs.md)). If the board lost track of it (its saved state is gone), it looks for the pane in the
  Coordinator's own folder, in whatever space that is: a Coordinator running there gets your message, in its current
  conversation, and an empty `Coordinator` tab left in that folder is reused rather than a second one opened.
- **`coordinator stop`, `restart` and `new` close its tab**, never the home space around it.

### Moving over from the old layout

The Coordinator used to have a space of its own named "Coordinator". One that still runs there is moved into the tab
as it is: the same program, mid-whatever it was doing, nothing restarted. Herdr closes the old space when its last pane
leaves.

- This happens by itself within a minute of the update (the keeper or an open board does it), or at your next message
  if that comes first.
- On a Herdr that will not move a pane, your next message closes its pane in the old space and starts it again in the
  tab, in the same conversation. `stop`, `restart` and `new` do the same.
- Only its own pane is ever closed. If you put panes or tabs of your own in that space, it stays open with those.
