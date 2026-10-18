# Talk to a coordinator

Use this page to send work to the Coordinator, to a machine coordinator or to a project coordinator. It also shows
how to restart, replace, stop and go into a coordinator.

For how a coordinator works and why, see [the Coordinator](../explanation/coordinator.md) and
[machine and project coordinators](../explanation/machine-and-project-coordinators.md).

## Send a message to the Coordinator from the board

The Coordinator is the agent in the board's chat. It changes what you type into tasks.

**Note:** Your first message starts the Coordinator. If its program stopped or Herdr restarted, your next message
starts it again in the same conversation.

1. Type your message in the chat's text box.
2. Send the message:
   - To send it, press `Enter`. The Coordinator adds the tasks. It delegates them only if you ask for that in the
     message.
   - To send it and start an agent on every task that it adds from that message, press `Tab`.

To show or hide the latest message of the Coordinator on the board, press `Alt+C`.

## Send a message to a machine coordinator

1. Press `Alt+M` to show every machine on the board.
2. Move to the heading of the machine.
3. Press `Space`. The chat opens the coordinator of that machine.
4. Type your message.
5. Press `Enter` to send it, or `Tab` to send it and start an agent on every task that it adds.

## Send a message to a project coordinator

1. Press `Alt+S` to group the tasks by repo.
2. Move to the heading of the repo. The tasks of the home folder also have a heading and a coordinator.
3. Press `Space`. The chat opens the coordinator of that repo.
4. Type your message.
5. Press `Enter` to send it, or `Tab` to send it and start an agent on every task that it adds.

The chat shows the name of the coordinator (`herdr-projects coordinator  idle`). The text box also shows it:
`Send message to herdr-projects coordinator`. Before your first message, the chat says
`not started: your first message starts it`.

**Note:** The chat follows the selection. If you move to another heading, you talk to the coordinator of that
heading. If you move to a task, you talk to the agent of that task.

**Note:** A status group heading (`to do`, `done`) has no coordinator. If you press `Space` there, the board tells you
so.

To go back to the Coordinator, press `Space` again.

## Send a message from a shell

Use the `coordinator` command:

```
herdr-projects coordinator                     how it stands
herdr-projects coordinator tell <text>         send it a message
herdr-projects coordinator tell <text> --delegate   the same as Tab
herdr-projects coordinator restart             start it again, same conversation
herdr-projects coordinator new                 a fresh conversation
herdr-projects coordinator new [--agent KIND[/MODEL]] [--effort E]    a different one, this once
herdr-projects coordinator stop
```

To use a machine coordinator or a project coordinator, add `--machine` or `--project <repo>`. Every `coordinator`
command accepts these flags:

```
herdr-projects coordinator --machine                    how it stands
herdr-projects coordinator --machine tell <text>        send it a message
herdr-projects coordinator --project herdr-projects tell <text> [--delegate]
herdr-projects coordinator --project <repo> restart | new | stop
```

To see which repo and which task were kept with your last messages, use this command:

```
herdr-projects coordinator where
```

## Restart, replace or stop the Coordinator

Restart and new are commands, not board keys. You can also ask the Coordinator to do them for you.

- To start it again in the same conversation, run `herdr-projects coordinator restart`.
- To start a new conversation, run `herdr-projects coordinator new`.
- To use a different program, model or effort for the next conversation only, run
  `herdr-projects coordinator new [--agent KIND[/MODEL]] [--effort E]`. Your default setting does not change.
- To close its tab, run `herdr-projects coordinator stop`. Your next message starts it again in the same conversation.

**Note:** `stop`, `restart` and `new` close the tab of the Coordinator. They never close the home space around it.

**Note:** If you close the board, the Coordinator does not stop. The board tells you this when you close it.

To change the program, the model or the effort for all conversations, change the `coordinator.agent` setting and the
other `coordinator.*` settings. See [roles](../reference/roles.md) and
[change a setting](change-a-setting.md).

## Go into the Coordinator in Herdr

On the board, you see only what the Coordinator said and what you said. To see the whole conversation, its dialogs,
its slash commands and its menus, go into its pane in Herdr.

1. Go to the `Coordinator` tab of your home space (`~`). You can also use the plugin action
   `herdr-projects.coordinator` from any place in Herdr. If the Coordinator does not run, this action starts it first.
2. Do your work in the pane, as with any agent.
3. To go back to the board, press your board key or run `herdr-projects`.

**Note:** To use the plugin action with one key, bind a key to `herdr-projects.coordinator` in Herdr.

**Note:** You can rename the `Coordinator` tab. The board finds the Coordinator by its pane, not by the name of the
tab.

## Related pages

- [The Coordinator](../explanation/coordinator.md)
- [Machine and project coordinators](../explanation/machine-and-project-coordinators.md)
- [Make an automation](make-an-automation.md)
- [Commands](../reference/commands.md)
- [Keys](../reference/keys.md)
- [Roles](../reference/roles.md)
- [Change a setting](change-a-setting.md)
