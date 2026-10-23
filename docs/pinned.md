# Pins and the Pinned tab

**Pin what matters to you, and step outside everything else.** A task, a project or a whole machine can be pinned.
Pinning moves nothing: the row stays where it always sorts and gets a small pin mark, `⚑`, after its name. The
**Pinned** tab beside the board shows only what you pinned.

```
 herdr-projects   Board 3   Pinned   proj · PRJ
 ○ ▾ vm-str  2 projects · 4 tasks
   ○ ▾ gtm · GTM ⚑
     ? GTM-14  Pick the ICP                                       2m ago
       Which segment first?
     ○ GTM-15  Write the first sequence                            1h ago
   ○ ▾ herdr-projects · HP
     ◐ HP-182  Pin machines and projects ⚑                ██░░ 40%  5m
```

## Pinning

- **`Ctrl+P`** pins the selected row, and unpins it: a task, a project's row (grouped by repo, `Alt+S`, or under
  Coordinators) or a machine's row (with every machine shown, or the one machine shown at the top). Its row says
  `Pinning…` while it goes ([under way](under-way.md)).
- **Nothing moves.** The task stays in its group, the project and the machine in their places; the selection stays on
  the row. The mark is the same for all three, dimmed as a tag is.
- **From the shell:** `herdr-projects pin <id | machine | project>` and `herdr-projects unpin …`. A task by its id or
  tag, a machine by its name, a project by its name or its key; `--on MACHINE` for another machine's project, and
  `vm-b:web/7` for another machine's task. `pin` pins whatever it was, `unpin` unpins.
  `herdr-projects list --pinned` lists what is pinned on this machine: the machine, its projects, its tasks.
- A pinned done task is never archived by `archive --done`. An archived task or project cannot be pinned; a pinned
  project that is archived leaves the Pinned tab until it is restored.

## The Pinned tab

`Alt+I` (or a click on `Pinned` in the header) shows it, and back to the board. The board remembers which tab shows,
also after it closes.

- **Only what is pinned:** a pinned machine with its projects and tasks under it, a pinned project with its tasks, a
  pinned task alone. Each stands under its machine's and its project's rows, so it is clear where it lives, and
  nothing is shown twice.
- **Everything works as on the board:** the same marks and colours, what needs you in red, and every key: talk to an
  agent, put one on a task, approve, stop, archive, a coordinator's row and its keys.
- **Always grouped by machine and project**, with no filter row: `Alt+S` and `Tab` say so and change nothing. Its
  folded headings are its own; the board's grouping, filter and folds are kept for when you go back.
- **Whose board** (`Alt+M`) counts here too: with every machine shown, every machine's pins.
- **Done tasks fold away** as on the board, pinned ones too: one line counts them (`↓ 2 done`), and `Alt+D` shows
  them.
- **Nothing pinned:** one line says how to pin something.
- **What needs you is never out of sight.** Pinning moves nothing, so all of it stays on the board. The tab that is
  not shown says in red how many of its rows need you: `Board 3` while you look at the Pinned tab. On the web board
  the pin button carries that count, and in the Pinned tab a red `3 need you on the board` takes you back.

## Where pins live

A task's pin is its own, kept with the task. A machine's and its projects' pins are kept on that machine
(`pins.json` in its data folder) and sent with its tasks, so every board that shows the machine sees the same pins,
after a restart too. Pinning another machine's project or the machine itself asks that machine to do it, as its
tasks' pins are. A machine on an older version cannot: the board says to update it there.

The code: `crates/board/src/board/update/pinned.rs` (the tab and the key), `crates/core/src/store/pins.rs`,
`crates/flow/src/actions/pins.rs`, `crates/flow/src/remote/pins.rs` and `src/cli/pins.rs`.
