# GitHub's checks

This page tells how the board waits for the checks on GitHub before it merges a pull request. It also tells what the
board does with a fine-grained token, and when `gh` cannot read GitHub.

## GitHub's checks must pass first

With `merge.checks: github`, a pull request merges only after its checks on GitHub pass. This is true for a merge by
itself and for a merge on your yes, in every repo that the board merges in. With `merge.checks: none`, the board does
not look at the checks on GitHub.

| The checks on GitHub | What the board does |
| --- | --- |
| still run | It waits. The row tells the progress: `Waiting for GitHub checks · 2 of 3 passed`, with the board's own mark `◇` (nobody works on the task in the meantime). When the checks pass, it merges by itself. |
| failed | It merges nothing. The failure goes back to the agent like a failed check of the board itself, with the name of the failing check and the end of its log. This happens within `review.rounds`. A merge that you approved continues when the checks on GitHub pass on the repair. After the rounds, it waits for you. Then `approve <id>` refuses and tells why, as for a failed review, until you run `approve <id> --anyway`. |
| do not report, and still run after `check.timeout` | It waits for you, with that reason. |
| passed, or there are none | It merges. |

The board counts each check one time. It counts a workflow as one check for its jobs until GitHub lists them.
`details` names what the board waits for (`checksWaitingFor`).

## The board decides from what the repo expects

The board does not decide only from what GitHub lists at that time. It uses these rules:

- Each GitHub workflow of the branch that runs for this pull request must have its run on the commit. That run must
  be complete and must pass: every job of it and every member of a matrix. A job that GitHub has not listed yet also
  counts, as one check that must still pass.
- The workflows are in `.github/workflows`. The board reads their triggers as GitHub reads them. These are `push`
  and `pull_request`, their `branches` against the branch, `paths` against the changed files, and `types`.
- The board does not wait for a workflow that cannot run for the pull request. Examples are a workflow only for
  other paths, only into another branch, or only on tags.
- Each check that the target branch requires (by its protection or its rulesets) must be listed and must pass.
- A check that was listed on the commit before, and is not in the list now, has not passed.
- If nothing is listed, the task says `Waiting for GitHub checks to start`, where an outside CI reports in the repo.
  The board looks for it on the newest commit of the target branch, or on the other pull requests of that branch.
- If GitHub cannot answer a question, the answer counts as yes.
- All of these wait for `check.timeout` at most. Then they wait for you, with the reason.
- A repo with no checks at all merges with no checks listed.

## A fine-grained GitHub token is enough

A fine-grained token cannot have the Checks permission, because GitHub gives that permission only to GitHub Apps.
Thus GitHub refuses the list of checks of a pull request to such a token. The board then reads the checks from what
the token can read:

- **Actions**: the runs on the commit, each of their jobs and matrix members, and the log of a failed run;
- **Commit statuses**: an outside CI, and Vercel.

Give the token read access to those two permissions. Nothing asks for Checks. A refused list is not a login problem.
The same rules apply. The board waits while checks run. A failure goes back with its log. The board waits for a
workflow that did not start. All of this is within `check.timeout`.

- Such a token cannot read the check runs of another app that posts no commit status. The task merges when all that
  it can read has passed. It tells what it could not read
  (`Merged: … Not read: check runs of apps other than GitHub Actions …`).
- The branch can require a check that no job or status of the commit answers. The board waits for such a check while
  a check that it can read still runs. Then it names that check there too. GitHub itself still refuses a merge that a
  required check blocks.
- If the token cannot read Actions or Commit statuses either, the task waits for you with
  `Could not read GitHub's checks`. The message names the two permissions to give.
- The board names a check one time, in whichever way it read it. Examples are a workflow `Check` and its job `check`,
  or the runs of a check for the push and for the pull request. The board does not look again for what it remembered
  for an earlier commit, or what it read the other way.

## When gh cannot read GitHub at all

This happens when you are not logged in, or when there is no network. A merge that waits for its checks says so on
its row (`Waiting for GitHub checks · could not read them (…)`). It continues by itself when `gh` can read again.
After `check.timeout`, it waits for you with `Could not read GitHub's checks`.

### The environment of the keeper

Herdr can start the keeper of the board (after a reboot) without the environment of your login shell. Thus the board
takes these from a login shell:

- your `PATH`;
- the login variables of `gh` (`GH_TOKEN`, `GH_HOST`, `GH_ENTERPRISE_TOKEN`, …). The board never writes a token down.
- the SSH agent (`SSH_AUTH_SOCK`, `SSH_AGENT_PID`), so that its `git fetch` and `git push` over SSH work. It does this
  only when the keeper has no SSH agent of its own and the socket of the login shell is there.

## Related pages

- [How work gets done](how-work-gets-done.md)
- [Rules of the gate](../reference/gate-rules.md)
- [When a task is done](how-work-gets-done-clean-up.md)
- [Settings](../reference/settings.md)
