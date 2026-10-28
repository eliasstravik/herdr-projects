# Autopilot

- Every coordinator is an agent that does what you ask, standing requests too ("merge tasks as soon as they pass"):
  it wakes on news and acts on it when you asked it to handle such things, or tells you what matters. **Autopilot**
  is a switch at the top of its chat's ⋯ menu: on top of that it also works on its own, within what you asked for.
  What you switch stays, per space. A space's news goes to its
  coordinator, and to its machine's home space while that one is on Autopilot. Every space's news on every machine
  (needs you, ready, stalled, merged, main red, machine health) also reaches the main machine's home space, in one line
  each: it keeps the overview and picks what buzzes your phone; the space's own coordinator acts on it. While Autopilot is on, a small red pill
  at the chat's top right says so (on a phone after the subtitle, under the name), a tap on it opens the ⋯ menu, and
  the space's row in the sidebar has a small red tag (with a mouse, its dot alone); with it off neither shows. Alerts buzz only while the app is closed or in the
  background; with it on screen they show nothing.
- Autopilot answers task agents' questions it can answer from the brief or the space, has conflicts rebased and
  failed checks fixed, starts queued tasks and closes tasks whose work is over, adds small follow-up fix tasks for
  what reviews, proofs or a red main found, and switches itself off and tells you why when things keep breaking. It
  never deletes tasks, changes Settings, starts work nobody asked for, force-pushes, or merges past a brief's "ask me
  first". It has no limits of its own: the machine's max tasks at once already bounds it. New tasks a day, retries,
  how long a task may take and when to hand back are its own judgment: Shepherdr gives it the facts (news, the
  agents' own progress) and trusts the model with the rest. A limit you want goes in its HEARTBEAT.md in plain words
  ("at most 5 new tasks a day"), and the coordinator keeps it.
- Triggers: Autopilot's heartbeat and automations run on the same kind of trigger, a time rule, event rules, or both
  (`server/live/trigger.ts`). The time rule is an iCalendar recurrence rule (RFC 5545: FREQ, INTERVAL, BYDAY, BYHOUR,
  BYMINUTE, BYSETPOS, UNTIL or COUNT, EXDATE, DTSTART; read by the `rrule` package), or `once 2026-11-02 9:00`: every
  90 minutes, every other week, the last Friday of the month, start and end dates and skip dates. Event rules are news:
  a task added, ready, gone wrong, needing you, merged or stalled, main red, a machine struggling (heartbeat only), each
  with optional `space=`, `branch=` (a glob) and `title=` filters. Every time rule's times are in Settings → Time zone
  (`time.zone`, Everywhere; this machine's own zone by default). You never write a rule: ask a coordinator in your words
  ("every 30 minutes, 8 to 22 on weekdays") and it writes one (`--when '<rrule>'`, `--on '<events>'`); everywhere it
  shows in words, with its next three runs ("Every 30 minutes, 8:00-22:00, Monday to Friday (Stockholm)").
- Autopilot is a heartbeat, as OpenClaw's. Settings → Autopilot holds just three things, per space (or a machine, or
  Everywhere): the Heartbeat, a time rule (every 30 minutes, all day), shown in words with its next runs and a few
  presets (every 15, 30 or 60 minutes; workdays 8-18; anything else, ask the coordinator); Wakes at once on, its event
  rules (all news, or none); and the Checklist, its HEARTBEAT.md, which starts with a checklist for its role (a space,
  a home space, the main machine's home space). On each run of the time rule, while its coordinator is idle,
  Shepherdr sends it one fixed prompt in its own session: go through its HEARTBEAT.md (its standing checklist in plain
  words, in the prompt itself, so a coordinator on another machine has it too; edited in Settings → Autopilot or by
  the agent with `shepherdr heartbeat set`; kept in `~/.shepherdr/heartbeat` on the main machine) and act on what needs
  it, or reply only `<no-reply>`, which the chat never shows. A busy coordinator gets it once its turn is over; none
  while its HEARTBEAT.md is empty and no news waits. News its event rules name wakes the coordinator at once, so a news
  card gets its reply right after it; other news waits and comes with the next heartbeat (quiet nights: a time rule
  within the day's hours, and only what can't wait as its events).
- Automations are cron jobs for tasks: a space's task template (title, brief, agent) and its trigger: a time rule,
  events of its own space (or the space a rule names), or both. Each run adds a new task with a fresh agent; none runs
  in the coordinator's chat. Only agents add or change them (`shepherdr automation`), when you ask a coordinator in the
  chat. They list at the bottom of the space's tasks, paused ones too, each with its trigger in words, its next runs
  and its last runs: run it now, pause or remove it there. Automations from before triggers (`daily 9:00`, cron,
  `--when merged`) became time and event rules once.
