# Fix a task that is behind main

This page tells what to do when a task waits as `behind main`, because the board could not fetch main. For how the
fetch works, read [behind main](../explanation/troubleshooting-behind-main.md).

**Note:** A task that is `behind main` did not fail. The board checks and reviews it as usual. You cannot approve it
until it is up to date. The board tries again by itself each minute.

If you approved a merge and the fetch then fails, the board merges nothing. When the row is ready again, approve the
merge again.

## Find the reason

1. Select the task.
2. Press `Alt+P` to open the task panel.
3. Read the reason in plain words under **Behind main**.
4. Read what git itself said under **Details**.
5. Find the reason below, and do the steps under it.

## origin asked for a login

The board never types a password.

1. Make `git fetch origin` work in your checkout without a prompt. Use an SSH key that is loaded in the agent, or a
   credential helper.

## origin did not answer within 60 seconds

1. If git waits for a password or a passphrase, do the same as for a login: make `git fetch origin` work without a
   prompt.
2. If not, the remote is slow or busy. Wait: the next try can pass.

## origin could not be reached

The cause is the network or the server.

1. Do nothing. The board tries again.

## origin has no branch …

The main branch of the remote has a different name, or the branch that the task started from (`workspace.from`) is gone.

1. In your checkout, run `git fetch origin`.
2. Run `git remote set-head origin --auto`.
3. Or, if the branch is gone, push that branch again.

## git's lock file … stayed in the way

Another git command wrote the same ref all the time. Or a git command that stopped left its `.lock` file behind.

1. Make sure that no git command runs in that repo.
2. Delete the file that the reason names.

## origin is not a repository the board can read

1. Run `git remote -v` in your checkout. It shows where origin points.

## Related pages

- [Behind main](../explanation/troubleshooting-behind-main.md)
- [Fix a task that seems stuck](troubleshooting-stuck-task.md)
- [Troubleshoot the board](troubleshoot.md)
- [The task panel](../reference/task-panel.md)
