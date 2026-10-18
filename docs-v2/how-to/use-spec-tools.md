# Use a spec tool (OpenSpec)

Use this page to make the board work with a spec tool. A spec tool works with the board through settings: it needs no
special support.

This page uses [OpenSpec](https://github.com/Fission-AI/OpenSpec): proposal and specs, then build, then archive. You
set up another spec tool the same way.

## Set the order of work and the check

**Note:** With no target, `openspec validate` asks which one, on the terminal. The board runs the check with no
terminal to ask on. Thus a `validate` with no target fails every time.

1. Tell every task's agent in the repo the order of work, with `worker.instructions`:

   ```sh
   herdr-projects set worker.instructions "Use OpenSpec. Write the change's proposal and specs first, under openspec/changes/. Then build it. When it is built, run openspec archive for that change." --repo my-app
   ```

2. Make a task's work count as done only when its specs are valid, with `check.command`. Give `validate` a target:
   `--all`, `--changes` or the name of a change each work.

   ```sh
   herdr-projects set check.command "openspec validate --all --strict --no-interactive" --repo my-app
   ```

3. Optional: add your tests after the validation, with `&&`.
4. Optional: put `check.command` in the repo's own `.herdr-projects.json` instead.

**Note:** Worktrees are not a problem. `openspec/` is in the repo, so the branch of every task has it.

For the settings, see [settings](../reference/settings.md).

## Approve the proposal before the build

Often you want to say yes to the proposal before the build.

**Note:** With `plan.approve` on, the plan step **cannot write the proposal files**. Until the plan has its yes, the
agent runs where the repo is a read-only file system, and nothing in it changes
([a plan first](../explanation/plan.md)). The plan is `plan.md` in the board's own folder, never a file in the repo.

Two ways work:

| Way | How | You say yes to |
|---|---|---|
| **Approve the plan, then the proposal is written** | `set plan.approve you --repo my-app`, and the instructions say "the proposal and specs are your first commit" | the plan's text, in the task panel. The build then writes the proposal as its first commit, and the review checks the work against the plan |
| **Review the proposal in the pull request** | leave `plan.approve` off; keep `merge.by you` (built in) | the proposal and the code together, in the pull request, before the merge |

The first way stops a wrong idea before anybody writes code. The second way shows you the real proposal files.

### Get the proposal by itself first

1. Add a task for the proposal only, for example "write the OpenSpec proposal for …".
2. Merge that task.
3. Add the build as a task with `--after` ([chain tasks](chain-tasks.md)).

## Related pages

- [Get a plan before the build](get-a-plan-first.md)
- [Chain tasks with `--after`](chain-tasks.md)
- [A plan first](../explanation/plan.md)
- [Change a setting](change-a-setting.md)
- [Settings](../reference/settings.md)
