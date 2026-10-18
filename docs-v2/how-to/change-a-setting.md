# Change a setting

Use this page to change a setting for you, for one repo, for one task, or for the tasks of one automation. All the
settings are in [settings](../reference/settings.md).

## Change a setting with a command

1. Find the name of the setting in [the list](../reference/settings.md#the-list).
2. Select the place for the setting:
   - For every task, type `herdr-projects set <name> <value>`.
   - For one repo, add `--repo <name>`.
   - For one task, add `--task <id>`.
   - For the tasks that one automation makes, add `--automation <name>`.

   ```
   herdr-projects set merge.by itself                       you: every task
   herdr-projects set worker.agent codex --repo my-app      your entry for one repo
   herdr-projects set review.rounds 5 --task 12             one task
   ```

3. Examine the result:

   ```
   herdr-projects show 12                                   what task 12 runs on, and where each value comes from
   ```

If the board refuses the name or the value, it shows the nearest correct one as a hint. Nothing changes.

Note: The board refuses a board setting (`board.`, `coordinator.` or `watcher.`) for a task or a repo. Set it for
yourself without `--repo` or `--task`.

## Set a setting when you delegate a task

`delegate` takes any work setting for the task. Give the setting as an option:

```
herdr-projects delegate <id> --reviewer.agent claude --merge.by itself
```

If the board refuses an option, nothing on the task changes. `add` and `automations` also take any work setting by its
name ([commands](../reference/commands.md)).

## Put a setting back

To put a setting back to the value that it would have without your answer, use `unset` with the same place:

```
herdr-projects unset <name>  [--repo … | --task … | --automation …]   back to what it would be
```

Note: `unset` is not the same as `none`. `none` is a real answer. For example, `set workspace.setup none --task <id>`
cancels the setup of the repo for that task.

## Change a setting of a task on the board

1. Select the task.
2. Press `Ctrl+E`. The settings page of the task opens.
3. Press `↑` or `↓` to go to a row.
4. Press `Enter`.
   - A switch or a choice goes to its next value. The board saves it immediately.
   - For all other rows, type the value in the box at the bottom.
5. If you typed a value, press `Enter` to save it. To keep the old value, press `Esc`.
6. To put the selected row back, press `Ctrl+U`.
7. To go back to the task, press `Esc` or `Ctrl+E`.

The change is for this task only. The row then shows `*`. The page, its rows and its keys are in
[the task panel](../reference/task-panel.md#the-settings-page).

## Ask the Coordinator

You can also ask the Coordinator in your own words to change a setting
([talk to a Coordinator](talk-to-a-coordinator.md)).

## Related pages

- [Settings](../reference/settings.md)
- [How settings work](../explanation/settings.md)
- [The task panel](../reference/task-panel.md)
- [Choose a model and an effort](choose-a-model-and-effort.md)
- [Add your own keys](add-your-own-keys.md)
