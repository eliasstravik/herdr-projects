# Send tasks to another repo

Use this page to add a task to a different repo than the repo of your board, without a second board. The header of
the board names the repo that new tasks go to.

There are three ways. Use the one that fits your work.

## Open the popup from a pane of that repo

With `board.open popup`, the board opens in the repo of the pane that you are in. A board in a tab keeps the repo
that it opened in.

**Note:** This works only if the board opens as a popup. To set this, run `herdr-projects set board.open popup`.
See [Settings](../reference/settings.md).

1. Go to a pane of the other repo.
2. Press the board key.
3. Type the task.

## Tell the Coordinator which repo

The Coordinator works on every repo. You can name the repo from any repo.

1. Tell the Coordinator the task and the repo, for example: "add this to gtm-agent".

The Coordinator adds the task with `--repo`. You can do the same yourself:

```sh
herdr-projects add "Fix the login" --repo gtm-agent
```

See [Talk to a Coordinator](talk-to-a-coordinator.md).

## Talk to the coordinator of the repo

1. Press `Alt+S` to group the list by repo.
2. Select the heading of the repo.
3. Press `Space`.
4. Type what you want.

Your text goes to the project coordinator of that repo. The tasks that it adds are tasks of that repo. See
[Machine and project coordinators](../explanation/machine-and-project-coordinators.md).

## Related pages

- [Other machines and several boards](../explanation/other-machines.md#several-boards)
- [Talk to a Coordinator](talk-to-a-coordinator.md)
- [Machine and project coordinators](../explanation/machine-and-project-coordinators.md)
- [Task numbers](../reference/task-numbers.md)
- [Settings](../reference/settings.md)
