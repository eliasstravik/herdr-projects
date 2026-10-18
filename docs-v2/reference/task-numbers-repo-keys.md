# Repo keys

This page tells how a repo gets its key, and the rules for a key. The key is the first part of a task name, for
example `HSW` in `HSW-134` ([task numbers](task-numbers.md)).

## Every repo and folder has a key

Every repo and folder that the board knows has a key: a git repo, a plain folder, the home folder, `/`. A task in no
repo belongs to the folder where you added it. Thus it has the key of that folder (`HOME-3` for a task added in the
home folder).

## How the board makes a key

The board makes a key at once, from the name. This occurs the first time that the board sees a repo or a folder: when
a task is added there, when the board is opened there, or when the Coordinator files a task there. The board saves the
key as your entry for that repo, in `~/.herdr-projects/settings/config.json`:

```json
{ "repos": { "gtm-skills": { "repo": { "key": "SKL" } } } }
```

| Name | Key | How |
|---|---|---|
| `team-paperwork` | `TPW` | Three words or more: their first letters. A long word splits where a word would split: `paper` `work`. |
| `gtm-skills`, `open-crm` | `SKL`, `CRM` | Two words: the last word, which tells what the repo is. A short word stays as it is. A longer word becomes its first letter and the consonants after it. |
| `proj`, `tmp`, `repo` | `PRJ`, `TMP`, `REPO` | One word: the same rule. A word with too few consonants becomes its first four letters. |
| `proj-2` | `PRJ2` | A number in the name goes at the end. |
| your home folder, `/` | `HOME`, `ROOT` | |

- **Never a key that another repo on this machine has, or had.** If the best key is taken, the board tries the next
  way to write the name (`PRO` for a second `proj`). Then it tries the best key with a number (`PRJ2`, `PRJ3`, …). The
  same names always give the same keys.
- **Kept.** The board never overwrites a key that you set, or a key that the repo's own `.herdr-projects.json` gives.
  The board gives a key only to a repo that has no key.
- **Stand-ins.** Two repos can already give the same key: two entries written by hand, or the files of two repos.
  Then the repo that the board knew first keeps the key. The other repo uses a **stand-in** that the board makes.
  - A repo whose own file (or your entry) gives a key that another repo used before also gets a stand-in. That key
    stays with the other repo.
  - The board keeps the stand-in in `~/.herdr-projects/data/repo-keys.json`, never in your settings. Thus what you
    wrote stays as you wrote it.
  - `show --repo` shows the stand-in: `not used: another repo has or had the same key; its tasks go by BETA`.
  - The note of the board on upgrade also shows it: `BETA for beta (its own KEEP is proj's too)`.
  - If you set a different key for that repo, the stand-in goes.
- **On upgrade.** The first time this version runs, every known repo without a key gets one key, once. The next time
  the board opens, it tells which keys: `Every repo and folder has a short key now: HOME for exedev, ROOT for home. Change one: herdr-projects set repo.key <KEY> --repo <name>`.
  If you remove a key from your file by hand, the board gives a key again the next time it opens, and tells you in the
  same way.

## Where a key comes from

`repo.key` is a setting ([settings](settings.md)) with its own rule, because it is a fact about one repo. It is never
about one task or all repos. The board uses the first of these that gives a key:

**your entry for the repo → the repo's own file → the key that the board makes.**

The repo's own file is `.herdr-projects.json` in the repo. A key there goes with the repo. It counts where your entry
gives none:

```json
{ "repo": { "key": "HSW" } }
```

The board reads the file of the repo as it is in the folder of the repo now (your checkout), not from the branch of a
task.

To set, remove or show a key, see [change a repo key](../how-to/task-numbers-change-a-repo-key.md). The Coordinator
sets or changes a key when you ask, with the same command.

## Rules for a key

- **2 to 5 letters or digits, and the first one is a letter.** The board saves and shows the key in capitals: it saves
  `hsw` as `HSW`. The board refuses all other keys, and gives the reason.
- **`PR` is refused**, because on a row it means a pull request.
- **Never no key.** The board refuses `set repo.key none`, and tells how to go back to the key that it made. `unset`
  never leaves a repo without a key: the key goes back to the key of the repo's file, or to the key made from the
  name, and the board says which.
- The board refuses a key on a task (`--task`) and for yourself (no `--repo`), and tells how to set it.
- **Two repos, one key: refused.** `set` refuses a key that another repo has, and names that repo.
- `set` also refuses a key that another repo *had*, so that an old name never points to the task of another repo:
  `HS was herdr-projects's key and still names herdr-projects's tasks`. A repo can take back a key that it had itself.
- The file of a repo can still bring a key that another repo has, because nobody is asked when a file changes. Then
  these rules apply:
  - Your own entry wins over a file.
  - If two repos give the key in the same kind of place, the repo that the board knew first keeps it.
  - The other repo uses a stand-in, as above. `show --repo` tells why its own key is not used.

Thus a key never names the tasks of two repos.

## Related pages

- [Task numbers](task-numbers.md)
- [Change a repo key](../how-to/task-numbers-change-a-repo-key.md)
- [Settings](settings.md)
- [The home folder](home-folder.md)
