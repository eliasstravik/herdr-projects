# Spec tools (OpenSpec)

**A spec tool works with the board through settings: no special support is needed.** This page uses
[OpenSpec](https://github.com/Fission-AI/OpenSpec) (proposal and specs, then build, then archive); another spec tool is
set up the same way.

## The settings

```sh
herdr-projects set worker.instructions "Use OpenSpec. Write the change's proposal and specs first, under openspec/changes/. Then build it. When it is built, run openspec archive for that change." --repo my-app
herdr-projects set check.command "openspec validate --all --strict --no-interactive" --repo my-app
```

- **`worker.instructions`** tells every task's agent in that repo the order of work ([settings](settings.md)).
- **`check.command`** makes a task's work count as done only when its specs are valid. Put your tests after it with
  `&&`. It can also go in the repo's own `.herdr-projects.json` (`check.command`).
- **Give `validate` a target.** With none, OpenSpec asks which one, on the terminal. The board runs the check with no
  terminal to ask on, so it fails every time. `--all`, `--changes` or a change's name each work.
- **Worktrees are fine**: `openspec/` is in the repo, so every task's branch has it.

## Approving the proposal

You probably want to say yes to the proposal before the build. One thing to know first: with `plan.approve` on, the plan
step **cannot write the proposal files**. Until the plan has its yes, the agent runs where the repo is a read-only file
system, and nothing in it changes ([a plan first](plan.md)). The plan is `plan.md` in the board's own folder, never a
file in the repo.

Two ways work:

| Way | How | You say yes to |
|---|---|---|
| **Approve the plan, then the proposal is written** | `set plan.approve you --repo my-app`, and the instructions say "the proposal and specs are your first commit" | the plan's text, in the task panel. The build then writes the proposal as its first commit, and the review checks the work against the plan |
| **Review the proposal in the pull request** | leave `plan.approve` off; keep `merge.by you` (built in) | the proposal and the code together, in the pull request, before the merge |

The first stops a wrong idea before any code is written. The second shows you the real proposal files. If you want the
proposal on its own first, make it its own task ("write the OpenSpec proposal for …"), merge it, and add the build as a
task with `--after` ([one task after another](after.md)).
