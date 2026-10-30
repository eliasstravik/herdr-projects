# Current model, effort and speed

A quiet line under every chat's message box says what its agent runs on, for example
`Opus 5.5 · medium · ⚡ Fast`, with no tap needed; tapping it opens the + menu, which has
the same Model, Effort and Speed. Normal speed is left out of the line. Native CLI
observations stay separate from selected launch settings: a configured model or catalog
default is not evidence of what a turn actually used. What was asked for at launch shows
only until a record says otherwise, marked as such (`Requested: Opus 5.5 · high`, per
field: `Opus 5.5 · Requested: high`). Not reported means nothing either way. Restart
keeps the recorded running agent and requested replacement distinct.

| | Model | Effort | Speed |
|---|---|---|---|
| Claude Code | assistant `message.model`, the `SessionStart` hook's `model` first | assistant top-level `effort` | assistant `message.usage.speed` |
| Codex | `turn_context` and `thread_settings_applied`, any hook's `model` first | `turn_context.effort`, `thread_settings.reasoning_effort` | `thread_settings.service_tier` |

Claude Code's own `/model`, `/effort` and `/fast` print a `local-command-stdout` record
at once, read as what they say; its next reply then says what really ran. So Fast asked
for on an account or model that doesn't allow it shows as Normal after the first reply.
Claude Code's `usage.service_tier` is not speed and is not read.

Codex writes `thread_settings_applied` at each turn's start and straight after `/fast` or
`/model`. Its API name is shown as the human word: `priority` is Fast, `default` is
Normal, though its config may say `service_tier = "fast"`. It records an effort only
when one was set; without one the model's own default runs, shown as Default.

A hook's time is when it fired, often after the turn it reports, so any transcript
record replaces hook data and a hook never replaces a record. Readers are tied to the
current launch and transcript, handle truncation and partial records conservatively,
and never read the terminal screen to guess state. A picker change without a subsequent
native record remains unobserved.

Speed can be picked in the restart sheet: Claude Code's `--fast-mode on|off` and Codex's
`-c service_tier="fast"|"default"`, both for that launch only, kept across later restarts
of the same chat. Unlike model and effort it is not a Setting, so it isn't saved for
future agents.

Shepherdr 2.1 launches Claude Code and Codex. Pi is a later addition. Remote logs and
hooks use the same core file/event interfaces as local agents, without an SSH log
mirror or daemon-proxy fallback. Native process liveness and explicit OSC status are
separate from task workflow: an idle or ended turn does not mean the task is Done.

The [acceptance record](core/implementation.md) names actual native versions, tested
status/restart cases and remaining provider/device limits. No hard token or money cap
is inferred from delayed native usage reports.
