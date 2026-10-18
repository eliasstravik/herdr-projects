# Machine and project coordinators

This page tells about the three levels of coordinator that you can talk to. Each level has its own conversation.
For the steps, see [talk to a coordinator](../how-to/talk-to-a-coordinator.md).

## The three levels

| Level | Talk to it | Named | Lives in Herdr |
|---|---|---|---|
| The [Coordinator](coordinator.md) | the chat, as always | `Coordinator` | a `Coordinator` tab in the home space (`~`) |
| A machine coordinator | `Space` on a machine's heading | after the machine: `vm-str coordinator`, `Mac coordinator` | a tab of that name in that machine's home space |
| A project coordinator | `Space` on a repo's heading | after the repo: `herdr-projects coordinator`, `~ coordinator` for the home folder | a tab of that name in the repo's own space |

The Coordinator does not change. The other coordinators are beside it. When you talk to one coordinator, the others
do not change.

## How the board chooses one

- **A machine heading** shows when the board shows every machine (`Alt+M`). `Space` on it opens the coordinator of
  that machine in the chat.
- **A repo heading** shows when the board groups the tasks by repo (`Alt+S`). `Space` on it opens the coordinator of
  that repo. The tasks of the home folder also have one.
- The chat shows the name of the coordinator (`herdr-projects coordinator  idle`). The text box also shows it:
  `Send message to herdr-projects coordinator`. `Enter` sends. `Tab` sends and starts an agent on every task that it
  adds, as with the Coordinator.
- **The chat follows the selection**, as it does for the agent of a task. If you move to a different heading, you
  talk to the coordinator of that heading. If you move to a task, you talk to its agent. `Space` again goes back to
  the Coordinator.
- A status group heading (`to do`, `done`) has no coordinator. `Space` there tells you so.

Your first message starts a coordinator. Until then, the chat says `not started: your first message starts it`.

## What each one is

- **Each one is an ordinary agent in a chat that stays open**, as the Coordinator is. It has the same program, model
  and effort (`coordinator.agent`, …), the same board commands, and its own tab. Everything that you say is one
  conversation, and the board keeps it. If its program stops or Herdr restarts, your next message starts it again in
  the same conversation.
- **A machine coordinator** is about that machine as a whole: its repos and folders, and the tasks and agents on it.
  It runs in its own folder (`~/.herdr-projects/data/coordinator/<session>/machine/chat`). Its rules are in
  `AGENTS.md` in that folder, as the rules of the Coordinator are.
- **A project coordinator** is about one repo. It runs in the folder of the repo, at its main level: your checkout. It
  can read the repo freely. It changes nothing there unless you tell it to. Work goes on the board as tasks of that
  repo.
  - Its tab goes in the space of the repo, the space that Herdr labels with the name of the repo. If no such space is
    open, the board makes one there.
  - The `AGENTS.md` of the repo is yours. Thus its rules are in a file beside its state
    (`~/.herdr-projects/data/coordinator/<session>/projects/<repo>/rules.md`). The board gives that file when the
    coordinator starts, as it does for a working agent.
  - The board never uses your own chats in the folder of the repo as its conversation. It keeps the conversation that
    it has. It moves to a different conversation (after its own `/clear`) only when Herdr names that conversation for
    its pane.
- **The coordinators of another machine** run on that machine. The board of that machine sends what they said with
  its tasks, and what you type goes to that machine. A machine with an older version has no such coordinators. The
  chat tells you to update the version on that machine.
- **The sidebar of Herdr** names each coordinator as the board does. It shows its state and its last message.

## Related pages

- [Talk to a coordinator](../how-to/talk-to-a-coordinator.md)
- [The Coordinator](coordinator.md)
- [Other machines](other-machines.md)
- [List headings](../reference/list-headings.md)
- [Roles](../reference/roles.md)
- [The sidebar](../reference/sidebar.md)
