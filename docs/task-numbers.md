# Task numbers and repo keys

**Every task is called by its repo's short key and its number: `HSW-134`.** That is the name on every row and in every
message, never a bare `#134`. Its full id, `herdr-projects/134` (its repo's name and number), still works everywhere and
never changes.

- Every repo gets a key the moment the board first sees it, made from its name (`gtm-skills` → `SKL`).
- Change a key with `herdr-projects set repo.key HSW --repo herdr-projects`.
- Type `HSW-134` (any case) anywhere an id is taken: `herdr-projects delegate HSW-134`.

## The tag on a row

```
 ready to approve
 ! HSW-98  Add an undo for clear                                       R#2 PR#18 12m ago
           Fail · 1 critical, 1 major
 working
 ◐ HSW-134 Tidy the first-run questions               ███░░░░░ 40% 12m  R#1 PR#17  5m
           Running the tests
 to do
 ○ HSW-9   Rename the hint line                                                   2h ago
```

- **Where:** right of the status mark, left of the title: `HSW-134`, its repo's key and its number.
- **Quiet:** in the theme's grey for greyed-out words, the same on every row and in every state, so the title is what you
  read first.
- **One column, so every title starts in the same place.** The tags share one column as wide as the widest tag of the
  list (all its tasks, folded ones too, so it does not move as the list folds). A shorter tag is padded with spaces. So
  titles line up even where ids differ in length: `HP-1` lines up with `HSW-206`. Tags are written from the left:

  ```
   ○ HP-1      Tidy the first-run questions
   ○ CRM-2     Rename the hint line
   ○ HSW-206   Add an undo for clear
   ○ HSWXY-140 A task with a long key and a long number
  ```

  What a live task says, on its second line, starts under its own title, not under the tag.
- **Not the tags on the right.** `R#2` is the review round and `PR#18` the pull request: they sit at the right edge, in
  the colour of how the review and the checks went. The task's tag is at the left, grey, and has no `#`: a key is joined
  to the number with a dash.
- **Several repos in one list:** the key tells them apart. A tag is the repo's key with the number (`PW-12`) in every list, so two tasks numbered 1 in two
  repos read `PW-1` and `HSW-1`.
  To see each repo's tasks together, group the list by repo (`Alt+S`, in the `?` menu as "group by repo: see which repo
  each task is in"; the "Group by repo" button on the web board): each group's heading names its repo. The whole task
  names the repo too, in its `Task id` row (`pi-work/12`), with the session.
- **Other machines.** A task of another machine has the tag its own machine gives it (that machine's keys). Each machine
  counts its own tasks, so two machines can each have a `#1`. With every machine shown, each machine is a heading of its
  own with its tasks under it ([list headings](list-headings.md)), so a row has the plain tag (`HSW-1`), never its
  machine's name. Where there is no heading above (the board's messages, the chat, the whole task), it is named
  `otherbox:HSW-1`. Find takes the tag alone, without the machine. (A task of a machine still on an older version, whose
  repo has no key there, shows its number as that machine gives it: `#1`, `otherbox#1`.)
- **When room runs out**, the tag is the last thing to go before the rest of a name: after what a row says, the bar, the
  word `ago`, the end of a long name, the percent with the minutes left, the `R#` tag with the pull request, and the time
  ([the board](board.md) has the whole order). It goes for the whole list at once, so the titles stay in one column:
  when the row with the widest tag would have fewer than 16 cells for its name with nothing else beside it (a list under
  about 25 columns). Because the column takes a few cells from every row, each of the other things gives way that many
  columns earlier than it did.
- **On a narrow screen** (under 80 columns), where a live row's name wraps, the lines of the name, of what the task says,
  and of the bar all start under the title: a row has one left edge.

## Everywhere else a task is named

| Where | What it shows |
|---|---|
| the list, the archive, find: one-, two- and three-line rows, of every group | the tag, as above |
| the task panel and the details page | the tag before the title, greyed; the heading says `Task details  HSW-134 "its title"`. The id itself (`herdr-projects/134`) stands once, on the whole task, as `Task id` with the session: where a command needs it |
| the chat's line that names who you talk to | the tag between the mark and the task's name; a window too narrow to leave the status 20 cells goes without it |
| an automation's firings | `made HSW-140`, on the board and on the command line |
| the board's own messages | the tag and the title, in one form: `Archived HSW-134 "Fix the footer".`, `Pinned HSW-14 "…" to the top.`, `Stopped HSW-14 "…".`, `Removed …`. A message an action wrote with the id reads with the tag too (`HSW-14: approved, merging`). Only a line that shows a command to run keeps the id the command takes. Pinning says so, and an action on another machine says what it is doing there (`Pinning HSW-2 on otherbox…`), never a bare `…` |
| Herdr's sidebar, and a pane's title | the task's space is named with the tag and the task's title (`HSW-34 Add a file…`), which is the first row of its agent's entry ([Herdr's sidebar](sidebar.md)). The pane's title is the same, so the web UI's sidebar reads `HSW-34 Add a file…` too. A reviewer's pane is named the same way |
| the web board | the same tag on each row, in the same column and with the same limit on the room before a title. What a task says and its bar start under the title, not under the tag. A list too narrow to leave the widest tag's title 16 letters shows no tags, on every row at once. The tag stands before the task's title when one is open, and in the chat's line |

### The key beside a repo's name

Where the board names a repo, its key stands beside the name, dimmed: `gtm-agent · AGT`. That is:

- the board's top line, which names the repo new tasks go to (`herdr-projects  gtm-agent · AGT · 3 to do`, on the web
  board too);
- a repo's heading in the list grouped by repo (on the web board too);
- the `Task id` row of the whole task (`gtm-agent/12 · AGT`), and an automation's `Repo` row.

Never on a task's row, which starts with the key already (`AGT-12`). On a narrow screen a heading is cut at its end, so
the key goes first and the name stays.

### Short form or id

Everything that names a task in words uses the short form, the same everywhere:

| Where | With the key `HSW` |
|---|---|
| a command's answer and its error (`describe`, `delegate`, `archive`, …) | `HSW-134: updated (description)` |
| the board's messages, also what a command answered through the board | `HSW-135: restored` |
| what a task's agent is told about its own task | `its id on the work board is herdr-projects/134; the human also calls it HSW-134, and every command takes that name too` |
| a pull request's body, and the commit the board makes | `Task: HSW-134 (herdr-projects/134)` |

The id itself is kept where a program reads it, since it is the task's real name and the one that never changes:
`--json`, branch names, folders and paths (an id inside a path is never rewritten), and the commands an agent is given to
run. (The first column of `herdr-projects list` is the short name, lined up.)

## How a repo gets its key

Every repo and folder the board knows has a key: a git repo, a plain folder, the home folder, `/`. A task in no repo
belongs to the folder it was added in, so it has that folder's key too (`HOME-3` for a task added in the home folder).

**Made from the name, at once.** The moment the board first sees a repo or folder (a task is added there, the board is
opened there, the Coordinator files a task there), it makes a key from its name and saves it as your entry for that repo,
in `~/.herdr-projects/settings/config.json`:

```json
{ "repos": { "gtm-skills": { "repo": { "key": "SKL" } } } }
```

| Name | Key | How |
|---|---|---|
| `team-paperwork` | `TPW` | three words or more (a long word splits where a word would: `paper` `work`): their first letters |
| `gtm-skills`, `open-crm` | `SKL`, `CRM` | two words: the last one, which says what it is; a short word as it is, a longer one by its first letter and the consonants after it |
| `proj`, `tmp`, `repo` | `PRJ`, `TMP`, `REPO` | one word: the same; a word with too few consonants by its first four letters |
| `proj-2` | `PRJ2` | a number in the name goes at the end |
| your home folder, `/` | `HOME`, `ROOT` | |

- **Never a key another repo on this machine has, or once had.** When the best key is taken, the next way of writing the
  name is tried (`PRO` for a second `proj`), then the best one with a number (`PRJ2`, `PRJ3`, …). The same names always
  give the same keys.
- **Kept.** A key you set, or one the repo's own `.herdr-projects.json` says, is never overwritten: the board only gives a
  key to a repo that has none.
- **Stand-ins.** Where two repos already say the same key (two entries written by hand, or two repos' files), the repo
  the board knew first keeps it, and the other goes by a **stand-in** the board makes. So does a repo whose own file (or
  your entry) says a key another repo once went by: that key stays the other repo's. The stand-in is kept in
  `~/.herdr-projects/data/repo-keys.json`, never in your settings: what you wrote stays as you wrote it. `show --repo`
  says so (`not used: another repo has or had the same key; its tasks go by BETA`), and so does the board's note on
  upgrade (`BETA for beta (its own KEEP is proj's too)`). Set another key for that repo and the stand-in goes.
- **On upgrade.** The first time this version runs, every repo the board already knew without a key gets one, once, and
  the board says which when it next opens: `Every repo and folder has a short key now: HOME for exedev, ROOT for home. Change one: herdr-projects set repo.key <KEY> --repo <name>`. A key taken out of your file by hand is given again the
  next time the board opens, and said the same way.

## Changing a key

`repo.key` is a setting ([settings](settings.md)) with a rule of its own, because it is a fact about one repo, never about
one task or all repos:

**your entry for the repo → the repo's own file → the one the board makes.**

```
herdr-projects set repo.key HSW --repo herdr-projects     your entry for that repo
herdr-projects unset repo.key --repo herdr-projects       back to the repo's file's key, or to the one made from its name
herdr-projects show --repo herdr-projects                 what it is and where it comes from
```

Or, to let it travel with the repo, in the repo's own `.herdr-projects.json` (it counts where your entry says none):

```json
{ "repo": { "key": "HSW" } }
```

The Coordinator sets or changes a key when you ask, with the same command.

- **2 to 5 letters or digits, the first a letter**, kept and shown in capitals (`hsw` is saved as `HSW`). Anything else
  is refused with the reason. `PR` is refused: on a row it already means a pull request.
- **Never none.** `set repo.key none` is refused, and says how to go back to the made key. `unset` never leaves a repo
  without one: it goes back to the repo's file's key, or to the one made from the name, and says which.
- It is refused on a task (`--task`) and for yourself (no `--repo`), with how to set it.
- **Two repos, one key: refused.** `set` refuses a key another repo has, and names that repo. It also refuses a key
  another repo *had* (`HS was herdr-projects's key and still names herdr-projects's tasks`), so an old name never comes to
  point at another repo's task. A repo may take back a key it had itself.
- A repo's own file can still bring a key another repo has (nobody is asked when a file changes). Then your own entry
  wins over a file, and of two repos that say it in the same kind of place, the one the board knew first keeps it. The
  other goes by a stand-in, as above, and `show --repo` says why its own key is not used. So a key never names tasks of
  two repos.
- The repo's file is read as it is in the repo's folder now (your checkout), not from a task's branch.

## The key names the task

`HSW-134`, in any letter case, is accepted wherever an id is: every command (`details`, `delegate`, `tell`, `stop`,
`approve`, `review`, `retry`, `fix`, `finish`, `report`, `describe`, `pin`, `archive`, `restore`, `clear`, `show`,
`set … --task`). So "start HSW-134" is `herdr-projects delegate HSW-134`. The Coordinator's rules and the skill say so too.

The id keeps working and stays the task's real id: nothing is stored under a key. Changing a key breaks nothing: the id
still names the task, and so does the old key (`HS-134` after `HS` became `HSW`), however the key changed: by `set`, by
hand in your settings, or in the repo's own file. The board puts every key a repo goes by on record, in
`~/.herdr-projects/data/repo-keys.json`, as that repo's for good: no other repo may take it, by `set` or by its file.

## Finding a task

In find (`Ctrl+F` on the board, the search box on the web board), a task is found by its number and tag as well as by its
words: `134`, `#134` and `hsw-134` all match it. What you type still filters as you type (`13` also shows `HSW-13` and
`HSW-135`). When it is exactly one task's number or tag, the selection goes to that task. `herdr-projects list --query`
matches the same way.
