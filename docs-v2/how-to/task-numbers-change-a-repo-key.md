# Change a repo key

This page tells you how to set, remove and show the key of a repo. The key is the first part of each task name, for
example `HSW` in `HSW-134`. For the rules of a key, see [repo keys](../reference/task-numbers-repo-keys.md).

**Note:** A key must have 2 to 5 letters or digits, and the first one must be a letter. `PR` is not permitted. A key
that another repo has, or had, is not permitted.

## Set a key for one repo

1. Type the command with the new key and the name of the repo:

   ```
   herdr-projects set repo.key HSW --repo herdr-projects
   ```

2. Read the answer. If the board refuses the key, it gives the reason.

The key is now your entry for that repo.

**Note:** The old key continues to name the tasks of the repo. The id (`herdr-projects/134`) also continues to work.

## Set a key that goes with the repo

1. Open the file `.herdr-projects.json` in the repo.
2. Add the key:

   ```json
   { "repo": { "key": "HSW" } }
   ```

3. Save the file.

**Note:** Your own entry wins over this file. The file counts only where your entry gives no key.

## Remove your key

1. Type the command:

   ```
   herdr-projects unset repo.key --repo herdr-projects
   ```

2. Read the answer. It says which key the repo now has: the key of the repo's file, or the key made from its name.

**Note:** `set repo.key none` is refused. A repo always has a key.

## Show a key

1. Type the command:

   ```
   herdr-projects show --repo herdr-projects
   ```

   The answer shows the key and where it comes from.

2. If the repo uses a stand-in, read why: `not used: another repo has or had the same key; its tasks go by BETA`.

## Ask the Coordinator

You can also ask the Coordinator to set or change a key. It uses the same command.

## Related pages

- [Repo keys](../reference/task-numbers-repo-keys.md)
- [Task numbers](../reference/task-numbers.md)
- [Change a setting](change-a-setting.md)
- [Talk to a Coordinator](talk-to-a-coordinator.md)
