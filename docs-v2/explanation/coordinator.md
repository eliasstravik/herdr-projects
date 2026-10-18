# The Coordinator

This page tells what the Coordinator is, how it works and why it works that way. For the steps to use it, see
[talk to a coordinator](../how-to/talk-to-a-coordinator.md).

## One long conversation

**The Coordinator is the agent that you talk to in the chat of the board.** You type what you need, and it changes
that into tasks.

The Coordinator is one agent chat that stays open. It is not one call for each message. Everything that you type goes
into the same conversation. Thus a message such as "delegate that one too" works, and the Coordinator can ask you a
question back.

Each machine and each repo also has its own coordinator, beside this one. Press `Space` on its heading to talk to it.
See [machine and project coordinators](machine-and-project-coordinators.md).

## Your message, and where you were

Your words go to the Coordinator exactly as you typed them. The board keeps two more facts with each message: the repo
that you were in and the task that you had open. The Coordinator finds these facts itself.
`herdr-projects coordinator where` lists the last few messages, each with its own repo and task.

If you open a board from the pane of the Coordinator, that board stays in the repo that it was in last. The reason is
that the folder of that pane is not the folder of a repo.

## `Tab`: "and start it right away"

If you send a message with `Tab`, the board delegates every task that the Coordinator adds from that message. This is
true for any number of tasks, and you do not confirm anything more.

- **This does not depend on the memory of the Coordinator.** The message has one line under your words that says that
  you sent it with `Tab`. The chat of the board shows only your words. Also, the board itself does the delegation:
  `add` delegates each task that the Coordinator adds during the turn of a `Tab` message.
- A message that adds no task delegates nothing more. Examples are a question, a status check or a word about a task
  that exists.
- A message that you send with `Enter` adds its tasks. It delegates them only if you asked for that.

## What it says it did

If the Coordinator did something on the board, its message ends with one short line. That line names the task ids in
plain words: added, updated (a title, a description, settings), removed, started an agent, approved, archived,
restored. An example is "Added proj/12 and started an agent on it". The commands answer in the same words
([commands](../reference/commands.md)).

## Why you go into it

On the board, you see only what the Coordinator said and what you said. You never see the screen of its program. For
the whole conversation, its dialogs, its slash commands and its menus, you go into its pane in Herdr.

- It is an ordinary agent in the `Coordinator` tab of your home space.
- The plugin action `herdr-projects.coordinator` goes to its pane from any place in Herdr. You can bind a key to it.
  If the Coordinator does not run, the action starts it first.
- You go back as you do from any agent: with your board key, or with `herdr-projects`.

## What it is

- It is an ordinary Herdr agent, in a tab named `Coordinator`, in your home space (`~`).
- It runs in its own folder in the data folder of the board. `HERDR_PROJECTS_COORDINATOR_DIR` can name a different
  folder. Do not use that folder for other work, because the board writes the rules of the Coordinator there.
- It starts with the same arguments as a working agent: all its tools, with no limits.
- Your first message starts it, and it stays. If its program stopped or Herdr restarted, your next message starts it
  again in the same conversation.
- **If you close the board, the Coordinator does not stop.** The board tells you this when you close it. One
  Coordinator serves every board of the machine (a tab, the popup, the web board) and `coordinator tell`. Also, a turn
  of it can still be in progress. `herdr-projects coordinator stop` closes its tab. Your next message starts it again,
  in the same conversation.
- The board follows the new-chat command of the agent (`/clear`, `/new`). The chat and a restart use the conversation
  that the agent is in now.
- Its program, model and effort are settings (`coordinator.agent`, …). See [roles](../reference/roles.md).

## Its standing rules are a file

The rules tell the Coordinator what it is and how it uses the board. The board writes them as `AGENTS.md` in the
folder where the Coordinator starts: `~/.herdr-projects/data/coordinator/<session>/chat`
([the home folder](../reference/home-folder.md)). The program reads that file itself at the start of each
conversation.

Thus the Coordinator knows its rules in all these cases:

- after its own `/new` or `/clear`;
- after `coordinator restart` and `coordinator new`;
- after `coordinator new --agent <another kind>`, because the next program reads the same file;
- when you start the program in that folder yourself.

| Kind | Reads | So the board writes |
|---|---|---|
| Claude Code | `CLAUDE.md` (2.1.296 also reads an `AGENTS.md` beside it) | `AGENTS.md`, and a `CLAUDE.md` that only passes on to it (`@AGENTS.md`): the rules arrive once |
| Codex | `AGENTS.md` | `AGENTS.md` |
| pi | `AGENTS.md` | `AGENTS.md` |
| any other kind | not known | `AGENTS.md`, and one line before its first message that points at the file |

- **The files stay up to date.** The board writes the files again at each start and before each message. Thus the rules
  of a new version are in place under a Coordinator that runs. The Coordinator reads them at its next new chat or
  restart.
- **Your own notes stay.** The board owns only the block between its two marker lines. Text that you write above or
  below that block in `AGENTS.md` or `CLAUDE.md` stays, and the Coordinator reads it too.
- **One way for each kind, never two.** The Coordinator does not get a launch flag for its rules now. Before, it did:
  the appended system prompt of Claude and of pi, and `developer_instructions` of Codex. A file works the same for
  each kind and stays after the program stops. A flag stops with the process, and each person or program that starts
  the program must give it again. Also, the flag of Codex could not hold this much text on one launch line. With the
  file in place, a flag would give the same rules two times. Only a kind that the board does not know gets the
  pointing line too, because it can read `AGENTS.md` or not.
- **Working agents and reviewers are different.** They work in your repo, and the `AGENTS.md` of your repo is yours.
  Thus their rules still come with their launch.

## Where it lives in Herdr

The Coordinator has no space of its own in the sidebar. It is a tab named `Coordinator` in the home space, the space
that Herdr labels `~`. It is always a whole tab, never a pane in one of your tabs. In the list of agents of the
sidebar, its name is `Coordinator`. Its state shows as it shows above the box of the board (`Coordinator · idle`),
and its latest message shows under it ([the sidebar](../reference/sidebar.md)).

- **The board finds or starts it from any place.** A message can come from the board in a pane, from the board in the
  popup of Herdr, or from a shell (`coordinator tell`). It goes to the Coordinator that runs. If none runs, the message
  starts it again in the same conversation, in a `Coordinator` tab of the home space. This occurs if its program
  stopped, if its tab or the home space closed, or if Herdr restarted. There is never a second Coordinator, and the
  board makes no space for it.
- **If this machine has no home space,** the board makes one at your home folder, as Herdr does. Then it adds the tab.
- **Your view does not change.** The tab, and a home space that the board had to make, open without focus. You stay
  where you are. The board and all other tabs keep their size and layout.
- **Which space is home.** It is the space labelled `~`. Herdr labels a space with the folder of its first tab. Thus
  the label changes when you `cd` to a different folder in that tab. The board remembers the space where it last put
  the tab. It uses that space when no space has the label `~`. The board never uses the space of a task for the
  Coordinator.
- **You can rename its tab.** The board finds the Coordinator by its pane, as it finds the agent and the reviewers of
  a task ([tabs](../reference/tabs.md)). If the board lost track of it (its saved state is gone), it looks for the
  pane in the folder of the Coordinator, in any space. If a Coordinator runs there, it gets your message in its
  current conversation. If an empty `Coordinator` tab is in that folder, the board uses it again and does not open a
  second one.
- **`coordinator stop`, `restart` and `new` close its tab.** They never close the home space around it.

### Moving over from the old layout

Before, the Coordinator had its own space named "Coordinator". If a Coordinator still runs there, the board moves it
into the tab as it is. It is the same program, in the middle of its work, and nothing restarts. Herdr closes the old
space when its last pane goes out.

- This occurs automatically less than a minute after the update. The keeper or an open board does it. If your next
  message comes first, it occurs at that message.
- If your Herdr cannot move a pane, your next message closes the pane in the old space. Then it starts the Coordinator
  again in the tab, in the same conversation. `stop`, `restart` and `new` do the same.
- The board closes only the pane of the Coordinator. If you put your own panes or tabs in that space, the space stays
  open with them.

## Related pages

- [Talk to a coordinator](../how-to/talk-to-a-coordinator.md)
- [Machine and project coordinators](machine-and-project-coordinators.md)
- [The Watcher](watcher.md)
- [Roles](../reference/roles.md)
- [Commands](../reference/commands.md)
- [The home folder](../reference/home-folder.md)
- [The sidebar](../reference/sidebar.md)
- [Tabs](../reference/tabs.md)
