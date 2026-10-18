# Run long work in one repo

Use this page to do big work in one repo, for example a migration, a rewrite or a feature in many steps. The board has
no "initiative" or "project" object, and you do not need one.

Big work is a chain of ordinary tasks, a place that says where the work is, and a coordinator that keeps the whole
picture.

| You want | Use |
|---|---|
| one conversation that keeps the whole picture | the repo's **project coordinator** |
| one place that says the goal and what is left | a **tracking doc or pull request**, named in every task's description |
| steps that go in order | **batches as tasks**, chained with `--after` |
| the next step | usually a **fresh task and agent**; sometimes a word to the same task's agent |

## Plan the work with the project coordinator

The project coordinator is a long-lived chat about one repo. The board keeps it across restarts. It is beside the
Coordinator and never mixed with it ([machine and project coordinators](../explanation/machine-and-project-coordinators.md)).

1. To group the list by repo, press `Alt+S`.
2. Select the repo's heading.
3. Press `Space`.

   You talk to that repo's own coordinator.

4. Give it the big work, for example: "here is the migration, split it into batches and add them as tasks, each after
   the last".

   It reads the repo and adds the tasks. Next week, it remembers what it said about them.

One repo has one project coordinator. Two big pieces of work in the same repo share it. Thus, tell it which piece of
work you mean.

## Keep the whole picture in one place

An agent knows only its own task. Thus each task's description must name the place that has the whole picture. Use
one of these:

- **A doc in the repo**, for example `docs/migration.md`: the goal, the batches, what is done and what is left. Each
  batch's task says "read `docs/migration.md` first, and tick off your batch in it". The doc lands with the work, so the
  next agent reads it as it is now.
- **A pull request or issue on GitHub** that holds the plan, named by its link.

1. To write a description, run `herdr-projects describe <id> <text>`.

   The coordinator writes a description on every task that it adds.

2. To give words to every task of the repo, set `worker.instructions` for the repo:

   ```
   herdr-projects set worker.instructions "…" --repo <repo>
   ```

   For example: "this repo is mid-migration: read `docs/migration.md`" ([settings](../reference/settings.md)).

## Add the batches as a chain of tasks

Each batch is its own task, with its own check, review and merge. A small batch gets a good review. A whole migration in
one pull request does not.

1. Add the batches, each after the last:

   ```sh
   herdr-projects add "Migration batch 1: the models" --repo my-app --desc "Read docs/migration.md first."  # my-app/41
   herdr-projects add "Migration batch 2: the API" --repo my-app --after my-app/41 --delegate             # my-app/42
   herdr-projects add "Migration batch 3: the screens" --repo my-app --after my-app/42 --delegate
   ```

2. Start the first batch with `Tab`, or run `herdr-projects delegate my-app/41`.

Each other batch starts by itself when the batch before it is done. Its agent's first prompt says what came before:
the closing note of that task, and the pull request that it merged ([one task after another](../explanation/after.md)).

You can also tell the coordinator "then", "once that is merged".

## Choose a fresh agent or the same agent for the next batch

**A fresh task, with a fresh agent, is the default.** It starts from the merged work, with a clean context, and gets its
own review. The tracking doc and the closing note that the board passes on tell it what came before.

**Tell the same task's agent** only for small things that belong to that task. Press `Space` on the task, or run
`herdr-projects tell <id> <text>`.

- **While the task is open** (working, in review, waiting for your yes), your words go into its conversation, and its
  change grows: the same branch, the same pull request, reviewed again.
- **When it is done and cleaned up**, a message starts a new round: a fresh worktree from main, a new pull request, and
  a new conversation. The agent does not remember the last round. Thus a new task does the same job, and keeps the
  history of each batch apart.

Do not keep one agent going for a whole migration. Its context fills up, and one very large pull request is
hard to review.

## Related pages

- [Chain tasks with `--after`](chain-tasks.md)
- [One task after another](../explanation/after.md)
- [Talk to a coordinator](talk-to-a-coordinator.md)
- [Machine and project coordinators](../explanation/machine-and-project-coordinators.md)
- [Settings](../reference/settings.md)
