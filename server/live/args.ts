// Reads a shepherdr command line: positional words and --options (repeatable ones collect into lists).
import { splitFacts, STATUS_MAX, SUMMARY_MAX, TITLE_MAX } from '../../shared/types.ts'

export interface Parsed {
  words: string[]
  opts: Record<string, string[]>
  flags: Set<string>
}

const FLAGS = new Set(['other', 'multi', 'json', 'project-coordinator', 'off', 'help', 'force', 'archived', 'machine', 'later', 'save'])

export function parseArgs(args: string[]): Parsed {
  const words: string[] = []
  const opts: Record<string, string[]> = {}
  const flags = new Set<string>()
  for (let i = 0; i < args.length; i++) {
    const a = args[i]
    if (a.startsWith('--')) {
      const eq = a.indexOf('=')
      const name = eq > 0 ? a.slice(2, eq) : a.slice(2)
      if (eq > 0) (opts[name] ??= []).push(a.slice(eq + 1))
      else if (FLAGS.has(name) || i + 1 >= args.length || args[i + 1].startsWith('--')) flags.add(name)
      else (opts[name] ??= []).push(args[++i])
    } else words.push(a)
  }
  // --project, from before spaces, is --space
  if (opts.project) opts.space = [...(opts.space ?? []), ...opts.project]
  delete opts.project
  return { words, opts, flags }
}

export const one = (p: Parsed, name: string) => p.opts[name]?.at(-1)

/** a task's --summary and --facts: a facts line written at the summary's end comes apart; a summary past SUMMARY_MAX is
    refused, for its writer to make shorter (like a title past TITLE_MAX) */
export function summaryOpts(p: Parsed, rest: string[] = []): { said?: string; facts?: string } {
  const { said, facts } = splitFacts(one(p, 'summary') ?? rest.join(' '), one(p, 'facts')?.replace(/^\s*Facts:\s*/, ''))
  if (said.length > SUMMARY_MAX)
    throw new Error(
      `That summary is ${said.length} characters: write a shorter one, ${SUMMARY_MAX} at most, in plain words on what changed for the human. PR, commit, checks, review and what is not proven go in --facts "…"; more detail goes in your chat or your proof.`,
    )
  return { said: said || undefined, facts }
}

export const HELP = `shepherdr: talk to Shepherdr from a terminal (agents use it; so can you)

Spaces (a folder on a machine with a name and its own coordinator; each machine has its home space, ~)
  shepherdr space add <path> [--name NAME]       add a git repo folder as a space (its own herdr space); the name
                                                 follows the space rules: lowercase letters, numbers, - and _,
                                                 at most 80 characters, its own on its machine
  shepherdr space list
  shepherdr space rename <name> <new-name>       only the name that shows changes (its folder, herdr space and task
                                                 ids stay)
  shepherdr space remove <name>                  Shepherdr forgets it (its folder stays); not while its tasks work,
                                                 never the home space
  shepherdr mode <space> regular|autopilot       Autopilot off (regular) or on for a space's coordinator
  shepherdr heartbeat [<space>]                  a coordinator's heartbeat on Autopilot: its trigger in words (when:
                                                 heartbeat.when, the news that wakes it at once: heartbeat.on), when
                                                 last and next, and its HEARTBEAT.md (the standing checklist each
                                                 heartbeat brings)
  shepherdr heartbeat set [<space>] < file       change its HEARTBEAT.md: the whole new text on stdin (empty stops
                                                 the heartbeat); the human edits it in Settings → Autopilot too

Tasks (coordinators add them; only when the human asks, or on Autopilot)
  shepherdr task add "<title>" --brief "<the brief>" [--space NAME] [--after SHE-12,SHE-13] [--later]
                                                 [--set KEY=VALUE ...] [--agent CLI] [--model ID] [--effort LEVEL]
                                                 [--reviewer CLI] [--reviewer-model ID] [--reviewer-effort LEVEL]
                                                 the title at most ${TITLE_MAX} characters (it fits one line on a phone);
                                                 its agent starts at once, in its own worktree. Shepherdr ends the
                                                 brief with its Settings block (shepherdr show has the defaults); the
                                                 brief's own words win (skip the review, plan first, ask me before
                                                 merging). --set changes one line of it for this task, marked as set
                                                 by the coordinator: --set review=off, --set review.mustFix=critical,
                                                 --set review.rounds=1, --set merge="ask me first"; --set agent=…,
                                                 model, effort, reviewer, reviewer.model and reviewer.effort are the
                                                 flags below.
                                                 --later: its agent reads the brief and waits for a go (To do); the go
                                                 is a message to it that says so (task tell).
                                                 --agent, --model and --effort pick the CLI (claude, codex, pi, …),
                                                 model and effort for this task instead of Settings': only what
                                                 that CLI and model take (shepherdr models), or default.
                                                 --reviewer, --reviewer-model and --reviewer-effort pick who reviews
                                                 its change (codex, claude, pi), named in its brief's Review line.
                                                 --after queues it: its agent reads the brief and waits, and Shepherdr
                                                 tells it to go once those tasks are all done (merged, or nothing to
                                                 merge); vm-str:SHE-12 is another machine's.
                                                 While the machine has no room (its max tasks working, or its load
                                                 or memory over the start limit) a new task waits as Queued and
                                                 starts by itself once there's room, oldest first
  shepherdr task queue <id> --after SHE-12[,SHE-13]
                                                 queue a task after others, or change what it waits for; --after none
                                                 clears it and it goes. A message to a queued task reaches its agent,
                                                 which clears its queue itself if the message is a go
  shepherdr task list [--space NAME] [--json] [--archived]
                                                 archived tasks are left out; --archived lists only them
  shepherdr task show <id> [--json]
  shepherdr task tell <id> "<text>"            from a coordinator, a go to a task that waits (To do, Queued) waits
                                                 too while the machine has no room (Settings → the machine), and it
                                                 says so; Shepherdr hands it over once there's room
  shepherdr task stop <id>                       close its agent and herdr space; its work stays (a worktree with
                                                 unsaved or unmerged work is kept); writing to it starts it again
  shepherdr task close <id> [--summary "..."]    a task with nothing to merge: stop its agent, close its herdr space,
                                                 remove its worktree and branch (kept, and said so, when they hold
                                                 unsaved or unmerged work) and mark it Done (summary, at most
                                                 ${SUMMARY_MAX} characters: Nothing to merge)
  shepherdr task delete <id> [--force]           only when the human plainly asks: stop its agent, close its herdr
                                                 space (and all that runs there), remove its worktree, branch, chat
                                                 and record, so it leaves every list; posts nothing. Refused while it
                                                 holds uncommitted or unmerged work, unless --force. A merged task's
                                                 commits stay on main
  shepherdr task archive <id>                    only when the human asks: out of every list and count, its record,
                                                 chat and summary kept (search and Archived find it); a running
                                                 agent is stopped first. Done tasks archive themselves after
                                                 tasks.archiveAfter (default 7 days; off allowed)
  shepherdr task unarchive <id>                  bring it back into its lists
  shepherdr task pause <id>                      only when the human asks: interrupt its working agent (as Esc does;
                                                 it stays alive) and put it in To do, as shepherdr todo; its work,
                                                 worktree and chat stay, and any message to it (task tell) goes on
  shepherdr task pin <id>                        only when the human asks: show it in Pinned at the top of its
                                                 tasks (it keeps its status); archiving or deleting it drops the pin
  shepherdr task unpin <id>                      put it back in its own group
  shepherdr restart <space|id> [--agent CLI] [--model ID] [--effort LEVEL] [--save]
                                                 only when the human asks: start a space's coordinator or a task's
                                                 agent again on Settings' CLI, model and effort for its role (or
                                                 these: what shepherdr models lists, or default); --save makes them
                                                 the space's Settings too. A working agent is interrupted first. The
                                                 same CLI resumes its session; another takes over with the chat so
                                                 far (a task's: its brief, commits and changes too). Its worktree,
                                                 branch and waiting messages stay; a coordinator's tasks go on

Task agents
  shepherdr todo <id>                            the brief says to wait for a go: show as To do until one comes
  shepherdr ready <id> --summary "<what changed>" [--facts "<PR · commit · checks · review · not proven>"]
                                                 the summary at most ${SUMMARY_MAX} characters, in plain words for the
                                                 human (longer is refused); the facts go in the PR,
                                                 task show and the coordinator's news, not the card;
                                                 Ready to merge (done is its old name); a branch with no changes (an
                                                 answer, research) has nothing to merge and goes straight to Done
  shepherdr pr <id>                              on GitHub: push its branch and open its pull request (a draft until
                                                 ready) or push to the open one; run it again after new commits
  shepherdr checks <id>                          wait for GitHub's checks on its pull request (up to 9 minutes); merging
                                                 waits for them only when Settings say to
  shepherdr merge <id>                           squash-merge into main (on GitHub through its pull request, as it is:
                                                 rebased only on a conflict with main, without waiting for GitHub's
                                                 checks unless Settings say to, refused on ones already failed; then
                                                 main here is pulled), mark merged, then put away
                                                 its agent, herdr space, worktree and branch (keeping unsaved or
                                                 unmerged work); only when the brief or the human says so. It
                                                 answers within two minutes: run it in the foreground and wait
  shepherdr plan <id> --step "..." [--step "..."]
  shepherdr status <id> "<line>" --done <0-100> [--left 5m|1h20m]
                                                 how far along you are, in your own words: the task's status line (at
                                                 most ${STATUS_MAX} characters), its pie and the time left (it counts down
                                                 while you work). Run it at the start and at each natural step

Chat
  shepherdr ask "<question>" --choice "<label>" ... [--desc "<label>=<one line>"] [--other] [--multi]
  shepherdr alert "<title>" --body "<why now>" [--task <id>]
                                                 with --task: it says what that task needs and opens on its card

Secrets (API keys, tokens, passwords: used by name, never seen or printed)
  shepherdr secret list                          the names you can use here, and where each is kept
  shepherdr secret ask NAME --why "<what for>" [--force]
                                                 a masked box in your chat for the human; end your turn, and a
                                                 <secret name="NAME"> line says it is kept. --force: ask again for one
                                                 kept already (wrong or expired)
  shepherdr secret run NAME [NAME …] -- <command> [its words]
                                                 runs the command with each one in its environment ($NAME),
                                                 masked (********) in what it prints
  shepherdr secret env NAME [NAME …] [--file .env]
                                                 writes NAME=value lines to a file git ignores (readable by you only)
  shepherdr secret set NAME [--machine | --space NAME] < file
  shepherdr secret rm NAME [--machine | --space NAME]
                                                 for the human: keep one (its value on stdin) or remove it, Everywhere
                                                 (none), for this machine or one space; the most specific one wins

Artifacts (one self-contained HTML file, optional JSON data; checked at phone width, then shown in your chat)
  shepherdr artifact new <file.html> --title "<title>" [--data <file.json>] [--space NAME]
  shepherdr artifact update <id> [<file.html>] [--data <file.json>] [--title "<title>"] [--note "<what changed>"]
                                                 a new version of the same one; --data alone changes only its data
  shepherdr artifact check <file.html> [--data <file.json>]   pictures and errors, without showing it
  shepherdr artifact list [--space NAME]
  shepherdr artifact show <id>                   its versions and the files of its latest, to edit
  shepherdr artifact rm <id>

Settings and automations
  shepherdr show [--space NAME]                the app's few settings (how tasks work, each agent's CLI, model and
                                                 effort, auto-archive, Autopilot's heartbeat in words with its next
                                                 runs, the time zone, this machine's
                                                 limits and numbers now, alerts,
                                                 voice) as they are where it runs (its space, else this machine),
                                                 each with where it comes from, and the lines every brief ends with
  shepherdr models [CLI]                         each CLI's models and the effort levels each takes, as the CLI says
  shepherdr set <key> <value> [--machine | --space NAME]
  shepherdr unset <key> [--machine | --space NAME]
                                                 defaults are set Everywhere (none), for this machine (--machine)
                                                 or one space (--space); the most specific one set wins.
                                                 unset drops that level's own, so the level above it counts again.
                                                 Counts and limits take any number in their range
  shepherdr automation add "<name>" --task "<title>" [--brief "<brief>"] [--when "<time rule>"] [--on "<events>"]
                         [--agent CLI] [--model ID] [--effort LEVEL] [--space NAME]
                                                 each run adds that task to the space, with a fresh agent; it runs on
                                                 its time rule, its events, or both (at least one)
  shepherdr automation edit "<name>" [--name "<new name>"] [--task …] [--brief …] [--when … | off] [--on … | none]
                         [--agent …]

  Triggers: automations and Autopilot's heartbeat (shepherdr set heartbeat.when / heartbeat.on) take the same ones,
  their times in time.zone (Everywhere). Write them from the human's words; show them back in the words Shepherdr
  prints, never as written.
      time rule (--when, heartbeat.when): an iCalendar RRULE (RFC 5545), with DTSTART and EXDATE lines where needed
          FREQ=MINUTELY;INTERVAL=30                       every 30 minutes
          FREQ=MINUTELY;INTERVAL=30;BYHOUR=8,9,…,21;BYDAY=MO,TU,WE,TH,FR    every 30 min, 8:00-22:00, weekdays
          FREQ=WEEKLY;INTERVAL=2;BYDAY=FR;BYHOUR=16;BYMINUTE=0           every other Friday at 16:00
          FREQ=MONTHLY;BYDAY=-1FR;BYHOUR=9;BYMINUTE=0                    the last Friday of the month at 9:00
          DTSTART:20261201T090000 RRULE:FREQ=DAILY;COUNT=10 EXDATE:20261206T090000   from, how many, skip dates
        or once 2026-11-02 9:00, or off. Without DTSTART it counts from 2026-01-01 0:00 (every other week, every 7 min)
      events (--on, heartbeat.on): all news, none, or some split by commas, each with optional filters
          added · ready · failed · needs-you · merged · stalled · main-red · health (heartbeat only)
          space=<name> · branch=<glob, * any> · title=<words in the title>       ready, merged branch=release/*
        an automation hears its own space's news unless a rule names another space=
  shepherdr automation pause|resume|run|remove "<name>" [--space NAME]
                                                 run: add its task now
  shepherdr automation list [--space NAME]     with each one's last run

Setup
  shepherdr secret elevenlabs                    store the ElevenLabs key for voice calls (reads it from stdin)
  shepherdr doctor                               check every machine saved in herdr; put its helper there again (main machine)
  shepherdr status                               spaces, open tasks, herdr, and done tasks that kept work
                                                 (with a task's id: that task's progress, above)
`
