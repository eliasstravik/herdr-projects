# Make a research task

This page tells how to make a task that changes nothing in the repo and gives you a report. For all the facts, see
[research tasks and reports](../reference/research-tasks.md).

## Add a new research task

1. Run `herdr-projects add "<title>" --research`.

   Example:

   ```
   herdr-projects add "Why is the build slow?" --research
   ```

You can also ask the Coordinator clearly for research, a review or a report with nothing to change. For example, type
"look into why the build is slow and tell me". The Coordinator then makes the task research only by itself.

## Make a task research only later

**Note:** After the agent has started, the change counts only from the next start of the agent.

1. Do one of these:
   - Run `herdr-projects delegate <id> --research`. This also starts the agent.
   - Run `herdr-projects set research.only true --task <id>`.
   - On the settings page of the task (`Ctrl+E`), select the row `Research only` and press `Enter`.
2. If the agent was already started, delegate the task again, so that its agent gets the new rules.

**Note:** The board refuses to turn it on when the agent has changed the repo or the pull request is open. Undo the
change or close the pull request first. Or add a new task as research only.

## Make every new task in a repo research only

1. Run `herdr-projects set research.only true --repo <name>`.

To do this for all your tasks, run the command without `--repo`.

To make one task in that repo an ordinary task, run `herdr-projects set research.only false --task <id>`, or delegate
it with `--no-research`.

## Make an automation make research tasks

1. Do one of these:
   - Add `--research` when you make the automation.
   - Run `herdr-projects set research.only true --automation <name>`.

## Read the report

1. Select the task on the board.
2. In the task panel, open the link under **Report** (`Open report`).

You can also run `herdr-projects details <id>`. The link is under `report`. You can read the report while the task
still waits for your answers.

## See where the value comes from

1. Run `herdr-projects show <id>`.

To remove an answer again, run `herdr-projects unset research.only`.

## If the agent changed the repo

After `review.rounds` rounds, the task stops and waits for you. Then do one of these:

- To send it back one more time, press `Ctrl+R`.
- If you want the change after all, run `herdr-projects delegate <id> --no-research`. The task becomes an ordinary
  task and starts.

## Related pages

- [Research tasks and reports](../reference/research-tasks.md)
- [An agent that waits for you is not finished](../explanation/waiting-for-you.md)
- [Change a setting](change-a-setting.md)
- [Make an automation](make-an-automation.md)
