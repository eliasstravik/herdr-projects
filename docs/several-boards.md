# Several boards: repos, machines, sessions, Coordinators

**Short answer: one board per machine, and it holds every repo.** You don't need a board per project. Each other
machine has a board of its own, and you can look at its tasks from yours. Each herdr session on a machine has its own
Coordinator, but they all share the machine's one list.

| You want | What happens |
|---|---|
| several repos | one board, one list. Every task's tag names its repo (`PW-12`, `HSW-3`), `Alt+S` groups the list by repo, and the header names the repo new tasks go to: the one you opened the board in ([task numbers and repo keys](task-numbers.md)) |
| several machines | each machine keeps its own tasks, agents and Coordinator. `Alt+M` shows another machine's tasks, or every machine's, on the board you have open, and acts on them there. Work never moves between machines ([other machines](other-machines.md)) |
| several herdr sessions on one machine | one list, shared: every session's board shows every task. A task's agent runs in the session it was started in; a board in another session shows that task as `In session <name>` with what was last known, and asks you to open the board there to act on it. The keeper looks after the tasks of every session |
| several Coordinators | one per herdr session, never two in one session, and not one per repo: the one Coordinator works on every repo, and each message carries the repo you were in ([the Coordinator](coordinator.md)) |
| to open the board twice | the board key never opens a second board: it goes to the one that runs ([the board](board.md)). A popup and a board in a tab can run side by side, and a board you start yourself with `herdr-projects` in a pane works too: they all show the same tasks, each with its own selection and filter |
| two separate lists on one machine (say, work and personal) | there is no built-in way. Keep one board and use the repo groups (`Alt+S`) or find (`Ctrl+F`). `HERDR_PROJECTS_HOME` moves the board's folder ([the home folder](home-folder.md)); it is not made for running a second board beside the first: herdr's board key and its events use the folder herdr itself was started with |
