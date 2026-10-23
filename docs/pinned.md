# Pins and the Pinned filter

**Pin what matters to you, and see only that when you want to.** A task, a project or a whole machine can be pinned.
Pinning moves nothing: the row stays where it always sorts and gets a small yellow star, `★`, after its name.
**Pinned**, the last filter on the filter row, shows only what you pinned, as `Needs you` shows only what needs you.

```
 herdr-projects  proj · PRJ
 vm-str │ All 299   Coordinators 7   Needs you 1   Working 5   Ready 0   To do 11   Done 283   Automations 2   Pinned 2
 ○ ▾ vm-str  2 projects · 4 tasks
   ○ ▾ gtm · GTM ★
     ? GTM-14  Pick the ICP                                       2m ago
     ○ GTM-15  Write the first sequence                            1h ago
   ○ ▾ herdr-projects · HP
     ◐ HP-182  Pin machines and projects ★                ██░░ 40%  5m
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
  project that is archived leaves the Pinned filter until it is restored.

## The Pinned filter

`Pinned` is a filter like the others on the filter row, after `Automations`: `Tab` gets there, a click shows it, and
the board keeps it shown as it keeps any filter, also after it closes.

- **Only what is pinned, in its places:** a pinned task in its group, a pinned project with all its tasks, a pinned
  machine with its projects and tasks. The board's grouping (`Alt+S`) and its folded headings stay as they are: by
  repo a pinned project is its heading with its tasks under it (alone, `0 tasks`, when it has none); by status or by
  machine its row stands above the tasks, as a project that needs you does under `Needs you`.
- **Its count** is what you pinned, each pin once: the pinned machines, projects and tasks, as `list --pinned` lists
  them (with every machine shown, every machine's). The other tasks of a pinned project or machine show under it but
  add nothing to the count, done or not.
- **Everything works as everywhere:** the same marks and colours, what needs you in red, and every key.
- **Whose board** (`Alt+M`) counts here too: with every machine shown, every machine's pins.
- **Done tasks fold away** as under `All`, pinned ones too: one line counts them (`↓ 2 done`), and `Alt+D` shows them.
- **Nothing pinned:** one line says how to pin something.
- **What needs you is never out of sight:** the filter row's `Needs you` and its red count stay on the row beside it.
- **On the web board** it is the same filter, the last pill on the filter row.

## Where pins live

A task's pin is its own, kept with the task. A machine's and its projects' pins are kept on that machine
(`pins.json` in its data folder) and sent with its tasks, so every board that shows the machine sees the same pins,
after a restart too. Pinning another machine's project or the machine itself asks that machine to do it, as its
tasks' pins are. A machine on an older version cannot: the board says to update it there.

The code: `crates/board/src/board/update/pinned.rs` (the filter and the key), `crates/board/src/board/update/shows.rs`
(the filter row), `crates/core/src/store/pins.rs`, `crates/flow/src/actions/pins.rs`, `crates/flow/src/remote/pins.rs`
and `src/cli/pins.rs`.
