# Make an automation

Use this page to make, change, fire and remove an automation. An automation is a saved task plus a trigger.

For all triggers, flags and keys, see [automations](../reference/automations.md). For how automations work and why,
see [automations](../explanation/automations.md).

## Make an automation with the Coordinator

This is the easiest way to make an automation or to change its words.

1. Type what you want in plain words in the text box of the Coordinator. For example: "every Monday at nine, look for
   dependencies that are out of date", or "after every merge, update the changelog".
2. Press `Enter`. The Coordinator runs the `automations` commands for you.

**Note:** On the row of an automation, no task is selected. Thus what you type goes where it goes when no task is
selected: to the Coordinator. If no Coordinator is on, the board adds your text as a task word for word. In that case,
make the automation with the command.

## Make an automation on a schedule

The schedule uses the time zone of this machine.

1. Give the automation a name, a prompt and a schedule:

   ```
   herdr-projects automations add "Look for old dependencies" --prompt "Check every package for a newer version" --schedule "mon 09:00"
   ```

2. To make sure that it is correct, run `herdr-projects automations show "Look for old dependencies"`.

**Note:** There is no webhook trigger. To watch something outside the board, use a schedule. For example, make an
automation that looks at GitHub every ten minutes.

## Make an automation on a board event

1. Give the automation a name, a prompt and an event with `--when`. To use the tasks of all repos, add
   `--every-repo`:

   ```
   herdr-projects automations add "Release note after a merge" --prompt "Write the release note" --when merged --every-repo
   ```

2. If the automation must fire only for some tasks, add `--if`:

   ```
   herdr-projects automations add "Jira handoff" --prompt "Move the ticket to Done" --when merged --if 'branch = *KIC-*'
   ```

**Note:** Without `--every-repo`, only the tasks of the repo of the automation count. To choose the repo, use
`--repo NAME|PATH`.

## Give the automation its own settings

An automation can hold its own value for each work setting of a task.

- To set a value when you make it, add `--<setting> VALUE` to `automations add`.
- To make its tasks research only, add `--research`. To stop that, use `--no-research`.
- To set a value later, run `herdr-projects set <setting> <value> --automation <name>`.
- To remove its own value, run `herdr-projects unset <setting> --automation <name>`.
- To see each setting that its next task will have, and where the value comes from, run
  `herdr-projects show --automation <name>`.

You can also change the settings on the board:

1. Select the row of the automation.
2. Press `Ctrl+E`.
3. Select a row and press `Enter` to change it.
4. To put a row back, press `Ctrl+U`.

**Note:** An edit applies from the next firing. Tasks that the automation made before keep their settings.

## Change an automation

- To change its name, prompt or trigger, run `herdr-projects automations change <name>` with `--name NEW`,
  `--prompt TEXT`, `--schedule …` or `--when …`.
- To set or replace the condition, use `automations change <name> --if '…'`.
- To remove the condition, use `automations change <name> --if none`.

**Note:** A new `--when` without `--if` keeps the condition that the automation had. A `--schedule` has no condition.

## Turn an automation on or off

- From a shell, run `herdr-projects automations on <name>` or `herdr-projects automations off <name>`.
- On the board, select the row of the automation and press `Space`.

**Note:** When an automation is off, it never fires and keeps all of its data. When you turn on a schedule again, it
counts from that time.

## Fire an automation now

**Note:** An automation that is off never fires. The same guards apply as for a trigger.

- From a shell, run `herdr-projects automations run <name>`.
- On the board, select the row of the automation and press `Tab`.

## See what an automation did

- To see when it fired, what fired it, and the task that it made or why it skipped, run
  `herdr-projects automations firings <name>`.
- On the board, select its row and press `Enter` (or `Alt+T`). Use `↑` `↓` to pick a firing. Press `Enter` on a
  firing to go to its task. Press `Esc` to go back to the list.

## Remove an automation

**Note:** The tasks that the automation made stay.

- From a shell, run `herdr-projects automations remove <name>`.
- On the board, select its row and press `Ctrl+X`. The board asks you first, as it does when you remove a task.

## Related pages

- [Automations reference](../reference/automations.md)
- [Automations on the board](../reference/automations-board.md)
- [How automations work](../explanation/automations.md)
- [Talk to a coordinator](talk-to-a-coordinator.md)
- [Make a research task](make-a-research-task.md)
- [Change a setting](change-a-setting.md)
- [Settings](../reference/settings.md)
