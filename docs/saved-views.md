# Saved board views

Keep an all-repos board and a board beside each project's Coordinator. They share the same tasks; each saved view
keeps its own repo scope, filter, selection, group-by-repo setting and folded headings.

```sh
herdr-projects                         # existing global board
herdr-projects --repo alpha            # saved view repo-alpha
herdr-projects board --view focus --repo alpha
herdr-projects --view focus            # restore scope and preferences
herdr-projects --view overview         # new named global view
herdr-projects --view shared --repo alpha,beta
```

`--repo` replaces a view's saved repo scope. Without `--view`, its name is `repo-` followed by the repo names joined
with commas, lowercased, with characters outside `a-z0-9._-` replaced by hyphens. Use explicit names when two repo
names would produce the same slug. Explicit view names must be nonempty and contain only `a-z0-9._-`; paths and
uppercase names are refused. A new name without `--repo` starts with all repos; an existing name restores its scope.

Repo matching is exact and case-sensitive: `alpha` does not include `alpha-web`. A comma-separated list includes any
of its named repos, without duplicates. Scope applies to tasks, archive rows, counts and automations, including
remote rows when machine scope changes. Grouping by repo and choosing machines work within the repo scope.
New tasks use the first repo's registered folder. An unknown repo has no matching rows and a warning; sending new
work is refused if the first repo's folder is unknown or gone. Register a repo by adding a task from its folder.

To place a board beside a project Coordinator, use Herdr's pane split command or configure a tab command ending in
`-- herdr-projects --repo alpha`. For a named view, use `-- herdr-projects --view focus --repo alpha` the first time,
then `-- herdr-projects --view focus`. Open the all-repos board in another pane. This feature does not create panes
or change Coordinators, keepers, settings or the plugin launcher.

Named views live in the board's data folder as `view-<name>.json`, with `repos` alongside the existing preferences.
The tab view named `popup` uses `view-named/view-popup.json` to preserve the legacy popup file.
Named popup views live in `view-popups/view-<name>.json` to avoid collisions with tab view names. A popup without its
own saved preferences starts from its named tab view without its draft. Plain boards keep using `view.json` and
`view-popup.json`, without a `repos` field; old files load unchanged. The folder overrides in
[home-folder.md](home-folder.md) apply to these files too.

Use different names for independent boards. Two boards using the same name share a preference file: the last save
wins. Task changes are shared across every view. Saved names currently select terminal views; the web board keeps
its existing view-selection behavior. Repo scope uses names on each machine, so repositories named differently on
another machine will not match. This is a view filter, not an access-control boundary.

When sending to a remote machine's Coordinator, its existing dispatch rules choose the task repo; the saved repo
scope filters what comes back but does not change that remote dispatch contract.

The board of every repo can also name each task's repo on its row: [the repo column](repo-column.md).
