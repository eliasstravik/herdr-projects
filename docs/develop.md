# Develop

```sh
bun install
bun run demo          # the server with fictional data (the prototype's world), on :7787
bun run dev           # the screen with hot reload, on :5173, talking to the server above
bun run typecheck     # types, in seconds: all a task runs here before it merges
bun run test changed  # by hand only: the tests that reach what you changed from main
bun run test          # by hand only: every test, in parallel (about 30 s)
bun run check         # by hand only: types, tests, build
```

Agents run no tests on this machine, to keep its CPU for herdr and the agents: a task merges once `bun run typecheck`
passes. GitHub runs the whole check on every push to main (`.github/workflows/check.yml`), not on pull requests, to
save Actions minutes, and a red main reaches the coordinator, who has it fixed forward. Tests run by hand run at the
lowest priority (nice 19, idle disk, a systemd scope with the least CPU weight).
On a machine where herdr runs as a systemd user service, give it (and the server and agents it holds) the CPU first:
`systemctl --user set-property herdr.service CPUWeight=1000`.

Agents working on Shepherdr: [AGENTS.md](../AGENTS.md) says what they check before merging.

The demo source (`SHEPHERDR_SOURCE=demo`) holds a fictional world for reviewing every screen and case without real
agents. Your data lives in `~/.shepherdr` (`SHEPHERDR_HOME` to move it).

```text
server/            the web server, the demo source and the live source
  live/            herdr link, chats from logs (readers/), tasks (engine/), settings, machines (box, mirror, install), voice, browser
src/               the screen's kernel, layout and shared pieces
plugins/<name>/    each base plugin's parts (yours: ~/.shepherdr/plugins/<name>/)
skill/             what every agent is taught (shepherdr/), coordinators (shepherdr-coordinator/), task agents (shepherdr-agent/)
helper/            what Shepherdr puts on another machine: its shepherdr command, hook recorder, log copier, health, browser relay
bin/shepherdr      the command agents use
bin/agent-browser  agent-browser at the pinned version, for each task's shared browser
```
