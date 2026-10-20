# Worktree destinations

`workspace.destination` names the parent folder for new worktrees. Each task, attempt or planning workspace gets
its own directory there. The default, `none`, leaves the location to Herdr. It follows the normal work-setting
precedence: task → your entry for the repository → you → repository file → built in.

For a repository named `my-app`, either an absolute parent or a relative one can place task worktrees under the
primary checkout's `.worktrees` folder:

```sh
herdr-projects set workspace.destination /path/to/my-app/.worktrees --repo my-app
# Equivalent when /path/to/my-app is the physical primary checkout:
herdr-projects set workspace.destination .worktrees --repo my-app
herdr-projects set workspace.destination .worktrees --task my-app/12
```

The repository can also commit `{ "workspace": { "destination": ".worktrees" } }` in `.herdr-projects.json`.
`unset workspace.destination --repo my-app` removes your repository override; `none` explicitly asks for Herdr's
default location. Paths are literal folder names, with no shell, tilde or template expansion.

Relative paths start at the **physical primary checkout**, including when delegation originates in a linked
worktree or through a symlink. The parent folder is created if needed and resolved through symlinks. A directory
name comes from the task's branch with slashes replaced by dashes; occupied names get `-2`, `-3`, and so on.
Existing directories, files, symlinks and registered worktrees are never reused. Names are claimed atomically in
a hidden `.herdr-projects-reservations` folder below the parent, leaving the checkout path absent for creation.

Herdr receives the absolute directory as `worktree create --path`. Later attempts in the same Herdr space use
Git to create their own worktrees in the same configured parent. Planning uses the task's worktree, so it follows
the same policy. The low-disk guard checks the configured destination volume before creating a worktree.
Existing worktrees stay where they are: changing this setting does not move or remove them.
The setting does not change handoff, merge or cleanup policy, or who owns a workspace.

Before setup or a worker starts, the board records and checks the returned location. With a destination configured,
it requires the exact requested physical path, a linked checkout of the same repository and the requested branch.
An absent, different or unrelated checkout fails the start with the requested and returned paths. The diagnostic is
saved as `worktreeLocation` on the task; an unverified checkout is never adopted as its owned worktree.
Inspect the checkout and Herdr's `--path` support, fix the configuration if needed, then delegate again.
No worktree is moved or deleted on failure. Name reservations remain after a failed creation or crash; a retry
chooses another name. Reservations can be removed manually after inspecting the corresponding checkout.
