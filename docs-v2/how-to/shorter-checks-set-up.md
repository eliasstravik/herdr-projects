# Set a shorter check for fix rounds

This page tells you how to make the board run a shorter check on the later rounds of a task. For why and when the board
uses it, see [shorter checks on fix rounds](../explanation/shorter-checks.md).

## Before you start

- The board uses the shorter check only when the checks of GitHub guard the merge: `merge.checks` is `github`, the
  task has its pull request (`merge.pr`), and a workflow of the branch runs for that pull request.
- The shorter check gets the commit that the full check last passed on. Use it to run only what the change since
  that commit reaches.

## Set the shorter check

1. Open the `.herdr-projects.json` file of the repo.
2. In the `check` object, find `command`.
3. Beside `command`, add `again` with your shorter check. Use `$HERDR_PROJECTS_CHECKED` for the commit that the full
   check last passed on.

This repo uses these values:

```json
{ "check": { "command": "npm run check -- --all", "again": "npm run check -- --since $HERDR_PROJECTS_CHECKED" } }
```

The default of `check.again` is `none`: the full check runs on each round.

## Related pages

- [Shorter checks on fix rounds](../explanation/shorter-checks.md)
- [Change a setting](change-a-setting.md)
- [Settings](../reference/settings.md)
- [Make a check report its progress](check-progress-report.md)
