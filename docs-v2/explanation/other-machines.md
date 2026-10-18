# Other machines and several boards

This page tells how the board shows and acts on the tasks of other machines, and how one board holds every repo.
For the setup steps, see [Set up another machine](../how-to/set-up-another-machine.md).

## Each machine keeps its own tasks

Each machine keeps its own tasks. `Alt+M` shows the tasks of the other machines on your board. Each press goes to the
next view: this machine, every machine, then each other machine by itself. An action on a task of a different machine
runs on that machine, over SSH. Work never moves between machines.

The first item in the header of the board says which machines you see. It is in the accent colour, and the board
never cuts it. For example: `all machines: otherbox`, or `only otherbox`.

### Why SSH

The commands of the plugin on another machine go over the SSH connection of the plugin itself. The reason is that
Herdr has no command that runs the commands of a plugin on another machine.

## Several boards

One board on each machine holds every repo. You do not need a board for each project. Each other machine has its
own board, and you can look at its tasks from your board. Each Herdr session on a machine has its own Coordinator,
but all of them share the one list of the machine.

| You want | What happens |
|---|---|
| several repos | One board, one list. The tag of each task names its repo (`PW-12`, `HSW-3`). `Alt+S` groups the list by repo. The header names the repo that new tasks go to: the repo that you opened the board in. See [Task numbers](../reference/task-numbers.md). |
| several machines | Each machine keeps its own tasks, agents and Coordinator. `Alt+M` shows the tasks of one other machine, or of every machine, on the board that you have open. You can act on them there. Work never moves between machines. |
| several Herdr sessions on one machine | One list, shared. The board of each session shows every task. The agent of a task runs in the session where it started. A board in a different session shows that task as `In session <name>`, with what it last knew. It asks you to open the board in that session to act on the task. The keeper looks after the tasks of every session. |
| several Coordinators | One Coordinator for each Herdr session, never two in one session, and not one for each repo. The one Coordinator works on every repo. Each message carries the repo that you were in. See [The Coordinator](coordinator.md). |
| to open the board twice | The board key never opens a second board. It goes to the board that runs. A popup and a board in a tab can run side by side. A board that you start yourself with `herdr-projects` in a pane also works. They all show the same tasks, each with its own selection and filter. See [The board header and list](../reference/board-header-and-list.md). |
| two separate lists on one machine (for example, work and personal) | There is no built-in way. Keep one board, and use the repo groups (`Alt+S`) or find (`Ctrl+F`). `HERDR_PROJECTS_HOME` moves the folder of the board, but it is not made to run a second board beside the first. The board key of Herdr and its events use the folder that Herdr itself started with. See [The home folder](../reference/home-folder.md). |

To send a new task to a different repo without a second board, see
[Send tasks to another repo](../how-to/send-tasks-to-another-repo.md).

## What you see of another machine

- **Headings.** When every machine shows, each machine is a heading of its own. Its repos (or status groups) are
  under it. A row has its plain tag (`#1`). The heading above it names its machine. See
  [List headings](../reference/list-headings.md).
- **Automations.** The [automations](automations.md) of a machine come with its tasks. They are in the part of the
  list of that machine, as its tasks are: a group of their own after the tasks of the machine. When the list is
  grouped by repo, they are under their repo. They fold with the heading of the machine. You switch them on and off,
  fire them, remove them and change their settings on the machine that has them.
- **The chat.** For each task that someone is on, its machine sends the last few things said in the conversation. It
  reads them there from the saved conversation of the program. They come from the agent, or from the reviewer
  during a review. The chat and the task panel show them as they show the tasks of this machine, under the same
  lines that name the speaker. See [The task panel](../reference/task-panel.md).
- **Its Coordinator.** While the board shows one other machine by itself, your text goes to the Coordinator of that
  machine. The chat shows the conversation of that Coordinator. The machine sends the state of its Coordinator
  (`idle`, `working`, how many of your messages wait) and the last dozen things said. The box shows them under the
  same lines (`Coordinator:`, `You:`) and scrolls back through them. A machine with no Coordinator says so. There,
  your text becomes a new task.
- **A machine on an older plugin.** The board finds it by its missing version. Its new fields stay blank, and the
  board says "update herdr-projects on <machine>". It sends no chat. The last message of its agent still shows. For a
  reviewer or its Coordinator, the box says that the machine needs the update.

## When a machine does not answer

The board never shows a machine that does not answer as a machine with no tasks. In place of its tasks, on the
terminal board and on the web board, one line says so, with the reason:

```
vm-a: its tasks cannot be read: herdr-projects is not found there.
```

- In the view of every machine, the tasks of the other machines still show under that line.
- `Alt+M` also goes to the view of that machine, while it does not answer. There you get the rest: what came back,
  what it means and what to do. The web board keeps this text behind "Why, and what to do". If the text is longer
  than the screen, the arrows scroll it and a line says that there is more.
- The line stays in its place while the board asks again every half minute. The board never repeats it as a message.
- If you act on a task of a machine that stopped answering, the one message of that action says why.

For the reasons and what to do, see
[Repair a machine that does not answer](../how-to/set-up-another-machine.md#repair-a-machine-that-does-not-answer).

## Going into the agent of another machine

`Enter` on the task of another machine first makes that machine open the space of the task again. If the agent
exited, that machine starts it again, in the same conversation.

- **In a Herdr window.** The Herdr of that machine puts the pane of the agent in front, as `Enter` does for an agent
  on this machine. The board stays and says:

  ```
  vm-a: its agent is in front there. Pick vm-a in herdr's sidebar to go to it: “<its title>”; Local there brings you back here.
  ```

  Herdr has no command that switches a window to another
  machine. Thus you must do that one click in the machines sidebar yourself. The agent is there when you do.
- **To come back.** Click **Local** in the sidebar, or press `prefix+w` and pick a space of this machine. This brings
  you back to the board, as you left it. The board key does not bring you back while that machine shows, because
  Herdr runs the key there. It opens the board of that machine.
- **Outside a Herdr client** (plain SSH), or with a machine on an older plugin, your terminal attaches to that one
  agent (`herdr agent attach`). `ctrl+b q` brings you back to the board.

## The work settings of a task go with it

You can put an agent on the task of another machine, or send that machine words that add a task. Then your machine
finds what you set for that task: your own settings, and your entry for its repo. It sends these with the task.

The other machine keeps them on the task as what the task carried, apart from all else that the task tracks. There,
they stand where the "you" settings of that machine would be: under the settings of the task itself, and above the
file of the repo. `show <id>` on that machine says `sent from <your machine>`.

Board settings stay with each machine: how many agents run at once, the Watcher and the Coordinator. See
[Settings](settings.md).

## In the Herdr web UI

The web board shows other machines as `Alt+M` does. It acts on their tasks. It opens their agents in the view of
that machine in the web UI, if the web UI has that machine as one of its PCs. The terminal board also works in the
terminal view of the web UI, on this machine and on another machine. What works how, and what the board says when
something is missing, is in [The web UI](web-ui.md).

## Related pages

- [Set up another machine](../how-to/set-up-another-machine.md)
- [Machine order](machine-order.md)
- [Send tasks to another repo](../how-to/send-tasks-to-another-repo.md)
- [The board header and list](../reference/board-header-and-list.md)
- [List headings](../reference/list-headings.md)
- [The Coordinator](coordinator.md)
- [Machine and project coordinators](machine-and-project-coordinators.md)
- [The web UI](web-ui.md)
