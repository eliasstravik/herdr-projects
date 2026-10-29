# Settings

- Settings (sidebar, or `,` on a desktop) holds the few app settings, in two groups. What agents are told: How tasks
  work, only words in each new brief's Settings block (review, the Reviewer a brief suggests, Must fix, Review rounds,
  merge), each row showing the exact line it adds. How Shepherdr runs: the agent, model and effort tasks and
  coordinators start with (model:
  an id or alias, or default for the CLI's own; effort: default, lowest, low, medium, high, xhigh or max, each CLI
  taking it its own way) and tasks' backup agent, when Done tasks archive themselves, Autopilot's heartbeat (its trigger) and checklist, the time zone of every schedule, each machine's limits, the idle browser, Notify, alerts and voice. Its
  defaults are set Everywhere, for a machine or for a space (`shepherdr set <key> <value> [--machine | --space NAME]`, from any machine);
  the most specific one set wins. And, kept on each device, its Appearance: System (the device's own light
  or dark), Light or Dark, set before the first paint, with the browser's bar and artifacts following it. A phone
  lists the sections in groups, as iOS's Settings does, and opens one over the list; a wider screen keeps the list
  down its side.
- Secrets (How Shepherdr runs) holds the keys, tokens and passwords agents use, by name: add, replace and remove them
  in masked boxes, Everywhere, for a machine or for a space; the most specific one wins. An agent that needs one asks
  with `shepherdr secret ask NAME --why "…"`: its chat shows a masked box, and the agent is told by name once it is
  kept. Agents run commands with them (`shepherdr secret run NAME -- <command>`, which masks them in what the command prints) or write them to a file git ignores
  (`shepherdr secret env NAME --file .env`), and never see them in a chat, a terminal or a log. All of them are kept
  in one file on the main machine, readable by you only (`~/.shepherdr/vault.json`); a value goes to another machine
  only when an agent there uses one, over its SSH link.
