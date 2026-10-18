# GitLab

**A repo whose `origin` is on GitLab goes through GitLab, on gitlab.com and on a GitLab of your own.** The board then
does with a merge request all it does with a pull request on GitHub: it pushes the task's branch and opens a draft
merge request, reads its state, waits for its pipeline before it merges (`merge.checks`), merges it with
`merge.method` at the commit that was reviewed, sends a failed job back to the agent with the end of its log, and links
the merge request on the task. It uses [glab](https://gitlab.com/gitlab-org/cli), GitLab's own program, where it uses
`gh` for GitHub. Nothing changes for a repo on GitHub.

## Which repos

The host of the repo's `origin` decides, as it is written in `.git/config`:

- `gitlab.com`: GitLab.
- a host named in `board.gitlabHosts`: GitLab. Name your own GitLab there, by its name or its link:
  `herdr-projects set board.gitlabHosts git.corp.example` (several: `git.corp.example, https://gitlab.other.org`).
- anything else (`github.com`, a GitHub Enterprise host): GitHub, through `gh`, as before.

## Logging in

glab's own login is used, for the host of the repo's remote:

- `glab auth login --hostname git.corp.example` (or `glab auth login` for gitlab.com), once, or
- `GITLAB_TOKEN` (and `GITLAB_HOST`) in your login shell's environment: the board takes them from it as it takes gh's
  login ([how work gets done](how-work-gets-done.md)).

The token needs the `api` scope. A login that is missing or refused is said on the task as one for you to fix, as for
GitHub.

## What it means on GitLab

- **Checks**: the merge request's pipeline on its newest commit, as a whole (named `pipeline`) and each of its jobs.
  A job allowed to fail, or one left to be started by hand, blocks nothing. `merge.checks` `gitlab` is the same as
  `github`; `none` merges without looking at the pipeline.
- **Merge method**: `squash` squashes and `merge` merges, each the way the project is set to merge (a merge commit,
  semi-linear, or fast-forward). `rebase`, as on GitHub, first rebases the merge request onto the branch it merges into
  (only while it is still at the reviewed commit, and without running its pipeline again), then merges the rebased
  commits; on a fast-forward project that makes a linear history. Its source branch is deleted with the merge.
- **The shorter check** (`check.again`, [shorter checks](shorter-checks.md)): a fix round runs it when the merge request
  has a pipeline to run the full suite before the merge, a `.gitlab-ci.yml` in the task's worktree. Its `rules:` are not
  read: any `.gitlab-ci.yml` counts.
- **Words**: what the board writes on such a task, and what it sends its agent, says merge request, pipeline and the
  glab commands (`glab ci status`, `glab mr view --comments`), not pull request and gh. Its row names it `MR!12`.
- **Approvals**: a merge request that still needs an approval waits for it, and one where changes were requested goes
  back to the agent, as a review on GitHub does. Your own comments on it wake the agent; the board's comments do not.
- **Draft**: it is opened as a draft (`Draft:` in its title), and marked ready right before it merges.
