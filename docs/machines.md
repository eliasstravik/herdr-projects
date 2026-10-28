# Other machines

Shepherdr is installed once, on the main machine. Every other machine you save in herdr's Remote shows up on its own
(`herdr machine add <ssh target>`, which also sets up herdr there): its spaces (its home space at least), tasks and
chats, with their coordinators, run from the main machine. The other machine needs only herdr, git, perl and the agent CLIs (gh for
pull requests of GitHub repos); nothing to install by hand.

- How: the main machine runs each one from a worker process of its own (`~/.shepherdr/machines/<name>/`). Agents,
  panes, worktrees and live events go through herdr's own remote bridge; git, the Files and Diff panels and merges
  run over SSH (one shared connection); the agents' logs and Claude Code's hook events are copied over as they grow.
- The helper: Shepherdr puts a small folder there over SSH, `~/.shepherdr/helper/`: the `shepherdr` command for its
  agents (it reaches Shepherdr through an SSH tunnel from the main machine, so the API stays off the network), the
  hook recorder, the log copier, a health reader, the skills, and the pinned agent-browser with Chrome. A new machine
  gets it as soon as herdr has it saved; it is put there again whenever Shepherdr updates and its copy changed.
- `shepherdr doctor` (on the main machine) checks every machine, puts the helper there again, starts its worker again
  and says what is missing. Settings → Machines says how each one is: Ready, Putting the helper there…, Can't reach it
  (and why), or what it needs.
- A task's shared browser runs on the task's own machine, next to its app. Where that machine can run `tailscale
  serve`, your screen takes the browser's frames and sends your input straight to it over the tailnet (the main
  machine only hands out the link, under a path no one can guess); elsewhere they come through the main machine.
- A machine that can't be reached says `Can't reach` on its heading; its chats keep what they last showed and what you
  send waits until it is back.
- From before: a machine that had its own copy of Shepherdr reporting to this one gives its spaces, tasks and chats
  to the main machine the first time it is reached, and that copy is turned off in its herdr (`herdr plugin disable
  shepherdr`; its files stay). `config.json`'s `"main"` and `peers.json` are no longer used.

## Health and limits

Each machine watches its own health: every 30 seconds it reads its 1-minute load against its cores, memory in use,
how full home and /tmp are, a GPU's use (nvidia-smi, where there is one) and whether herdr answers. A small meter at
the top right of its home space's chat shows it (thin bars, the numbers in words where there is room; grey while all is
well, amber while new tasks wait, red past an alert limit), and a tap shows the numbers, the tasks at work against
the limit, herdr, and its settings. Settings → Each machine → its name holds its limits, each machine keeping its
own, scaled to its cores and memory: limit working tasks by Number of tasks, Machine load or Both (Both); max tasks
working at once (10); new tasks wait while the load is over 1× the cores or memory over 80%; its coordinator is told
when the load stays over 1.5× the cores for 2 minutes, memory or a disk is over 90%, or herdr stops answering.
- **Room for tasks** (Shepherdr enforces it): a task added, or told to go (a queued task's go, or a coordinator's
  `task tell` to one that waits), waits as Queued, `Waiting for a free slot` or `Waiting for the machine`, with no
  agent started yet, and starts by itself once there's room, oldest first; while load counts, one at a time, 30
  seconds apart. Only working tasks count. Your own message to a task starts it at once.
- **Alerts** (the coordinator decides): past a limit, a card shows in the machine's home space and its coordinator (and
  the main machine's home space's) gets the numbers and what uses the machine most, by task; once more when it
  is back under. The shepherdr-coordinator skill says what to weigh: pause or queue tasks, clean the disk, or tell
  you. Shepherdr never pauses a running task itself.
- **herdr gone**: its socket refusing is said plainly ("herdr isn't answering"), never `connect ECONNREFUSED …`; the
  machine's heading says `herdr down` (as it says a limit it is over), its chats say messages wait until it is
  back, and its home space's coordinator hears what happened once herdr answers again.
