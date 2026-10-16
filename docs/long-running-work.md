# Long-running work in one repo

**There is no "initiative" or "project" object on the board, and you don't need one.** Big work (a migration, a
rewrite, a feature in many steps) is a chain of ordinary tasks, a place that says where it stands, and a coordinator
that keeps the thread.

| You want | Use |
|---|---|
| one conversation that keeps the whole picture | the repo's **project coordinator** |
| one place that says the goal and what is left | a **tracking doc or pull request**, named in every task's description |
| steps that go in order | **batches as tasks**, chained with `--after` |
| the next step | usually a **fresh task and agent**; sometimes a word to the same task's agent |

## A project coordinator per repo

Group by repo (`Alt+S`) and press `Space` on the repo's heading: you talk to that repo's own coordinator. It is a
long-lived chat about that one repo, kept across restarts, beside the Coordinator and never mixed with it
([machine and project coordinators](machine-and-project-coordinators.md)).

Use it as the place where the big work is planned: "here is the migration, split it into batches and add them as
tasks, each after the last". It reads the repo, adds the tasks, and remembers what it said about them next week.

One repo has one project coordinator. Two big pieces of work in the same repo share it: say which one you mean.

## A tracking doc or pull request

An agent knows only its own task. So each task's description names the place that has the whole picture:

- **a doc in the repo** (`docs/migration.md`): the goal, the batches, what is done and what is left. Each batch's task
  says "read `docs/migration.md` first, and tick off your batch in it". It lands with the work, so the next agent reads
  it as it now is;
- **or a pull request or issue on GitHub** that holds the plan, named by its link.

`herdr-projects describe <id> <text>` writes a description; the coordinator writes one on every task it adds. For words
every task of the repo should get ("this repo is mid-migration: read `docs/migration.md`"), use
`herdr-projects set worker.instructions "…" --repo <repo>` ([settings](settings.md)).

## Batches as tasks, chained with `--after`

Each batch is its own task, with its own check, review and merge. A small batch is reviewed well; a whole migration in
one pull request is not.

```sh
herdr-projects add "Migration batch 1: the models" --repo my-app --desc "Read docs/migration.md first."  # my-app/41
herdr-projects add "Migration batch 2: the API" --repo my-app --after my-app/41 --delegate             # my-app/42
herdr-projects add "Migration batch 3: the screens" --repo my-app --after my-app/42 --delegate
```

Start the first with `Tab` (or `herdr-projects delegate my-app/41`). Each of the others starts by itself once the one
before it is done, and its agent's first prompt says what came before: that task's closing note and the pull request it
merged in ([one task after another](after.md)). Or tell the coordinator "then", "once that is merged".

## The next batch: a fresh agent, or the same one?

**A fresh task, with a fresh agent, is the default.** It starts from the merged work, with a clean context, and its own
review. The tracking doc and the handed-on closing note tell it what came before.

**Telling the same task's agent** (`Space` on the task, or `herdr-projects tell <id> <text>`) is for little things that
belong to that task:

- **While the task is open** (working, in review, waiting for your yes), your words go into its conversation and its
  change grows: the same branch, the same pull request, reviewed again.
- **Once it is done and cleaned up**, a message starts a new round: a fresh worktree from main, a new pull request, and a
  new conversation. The agent does not remember the last round, so a new task says it just as well, and keeps each
  batch's history apart.

Don't keep one agent going for a whole migration: its context fills up, and one huge pull request is hard to review.
