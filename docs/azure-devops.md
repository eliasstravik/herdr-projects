# Azure DevOps

A repo whose origin is at Azure DevOps Repos goes through the finish line as one on GitHub does: its branch is pushed,
its draft pull request opened, merged on your yes (or by itself) once its policies pass, and watched meanwhile. Then it
waits for the base branch's pipeline: the task is done only once that run passed for the merge commit.

The board knows the host from the origin's URL, in any form Azure DevOps gives a clone:

- `git@ssh.dev.azure.com:v3/<org>/<project>/<repo>`
- `https://<org>@dev.azure.com/<org>/<project>/_git/<repo>`
- the older `https://<org>.visualstudio.com/<project>/_git/<repo>` and `<org>@vs-ssh.visualstudio.com:v3/…`

Anything else is GitHub, as before.

## What it needs

The Azure CLI with its azure-devops extension, logged in, where the board runs:

```sh
az extension add --name azure-devops
az login
```

Without it (no `az`, no extension, no login) the task says so as a login the human must give, the way a missing gh
login is said: `Could not open the pull request`, with what az said and what to run.

## The pull request

- **Opened** at the finish line: the branch pushed, a draft pull request into the task's base branch (`workspace.from`),
  with the same title and description as on GitHub (`az repos pr create --draft true`). One the agent opened itself for
  the branch is taken on instead.
- **Shown** on the row (`PR#42`, green when its policies pass, red when one fails), by its link
  (`https://dev.azure.com/<org>/<project>/_git/<repo>/pullrequest/42`). The task panel adds a line of its reviewers'
  votes and its policies (`Azure  Reviewers: Ann approved · Policies: CI build running`); `details` has the pull
  request as Azure DevOps says it now (`azure`: its status, reviewers and policies) and, after the merge, the main
  pipeline's runs.
- **Merged** on approve: only at the commit that was reviewed, marked ready, its policies passed, then completed by
  `merge.method` (`squash`; `merge` is a merge with no fast-forward; `rebase`), its source branch deleted. The strategy
  is named on the completion, whatever one the pull request already had (an auto-complete set in Azure DevOps). It is
  Azure DevOps' REST call through `az rest` (`az repos pr update` can say neither the strategy nor the commit), so it
  needs `az login`: a token from `az devops login` alone does not do for it.
- **Completed by you in Azure DevOps**: the task is merged without the board's own merge, and needs no clean checkout.
  **Abandoned** while its merge was awaited: it waits for you, as a closed pull request does on GitHub.
- **Merged by the agent itself**: the task is done, `Already merged in !1234`.
- **Feedback**: what you write on it (as the user `az` is logged in as) wakes the agent, as on GitHub. The board's own
  review comment is a thread closed at once, so it never holds up a policy of resolved comments.

The pull requests are looked at on the same cadence as GitHub's, every two minutes.

## Its policies are its checks

`merge.checks` is `auto` by default: `github` on GitHub, `azure` on Azure DevOps. With `azure`:

| Policy | The board |
|---|---|
| a build validation or status check still running | waits, at most `check.timeout`: `Waiting for Azure DevOps checks · 0 of 1 passed` |
| a build validation held at an approval (its environment's check in Azure Pipelines) | waits for you, with no timeout: `Waiting for your approval in Azure DevOps: the build reports, at Staging`. Once you approve it there, it waits for the build again, `check.timeout` counting from then |
| one that failed | merges nothing; the failure goes back to the agent with the policy's name, the failing job and step, and the end of its log, within `review.rounds`, then waits for you (`approve <id> --anyway` merges all the same) |
| a policy of people not met (reviewers, required reviewers, resolved comments, linked work items) | waits for it, as a review still required on GitHub; a reviewer who rejects, or waits for the author, is a change asked for |
| one that is not blocking | never holds the merge up |

`merge.checks: none` does not look at them.

## The main pipeline

After the merge, the task stays at "merging" while the base branch's pipeline (`azure-pipelines.yml`, or whichever the
repo has) runs for the merge commit, its row saying how far: `Main pipeline running · 2 of 3 stages`, or
`Deploying staging` while a stage named `Deploy…` runs.

- **Passed**: the task is done and cleaned up. A run that partly succeeded counts as passed, and says so.
- **Failed or cancelled**: the task waits for you, with the run, the failing job and step and the end of its log.
  `fix` sends that to the agent; it fixes it on its branch (which has the merge in it) and finishes again, and a new
  pull request brings the fix in. `retry` looks at the runs again, after you ran the pipeline again in Azure DevOps (the
  newest run of a pipeline counts).
- **Held at an approval** in Azure DevOps (a deployment stage's environment check): the task waits for you,
  `Waiting for your approval in Azure DevOps: the pipeline reports, at Production_DE`, with no timeout; once approved
  there it goes on by itself.
- **Not through after `merge.pipelineTimeout`** (60 minutes by default, counted from your last approval there): it
  waits for you with that reason.

`merge.pipeline` says which runs it waits for:

| `merge.pipeline` | Waits for |
|---|---|
| `all` (built in) | every pipeline run of the base branch for the merge commit. A repo with no pipeline is done at once; one with none that ran for the commit after 10 minutes is done, saying so |
| a name or id, or several (`CI, deploy`) | those, each until it starts and passes |
| `none` | nothing: done at the merge |

The gate is Azure DevOps' only: on GitHub a task is done at the merge, as before.
