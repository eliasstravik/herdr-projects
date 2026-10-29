# Current model and effort

The composer + menu shows CLI observations (`Chat.agent.runtime`). `Chat.agent.model` and `effort` remain the selected
launch policy; missing/default values still inherit the CLI configuration. Restart's `on` and `want` retain their
existing meaning. Neither Settings nor Shepherdr's model catalog fills a missing runtime observation.

Both coordinator and task chats use the same path. Observations are tied to the active transcript and this launch's
start time. Old logs cannot supply a new launch's values; a resumed Codex thread can supply current state before its
next turn via the existing daemon. No thread is started or resumed to inspect it.

## Sources and limits

- **Codex:** `turn_context.model`, `effort` (older versions), `reasoning_effort`, and
  `collaboration_mode.settings.reasoning_effort`. A turn context replaces the previous snapshot, including clearing
  unset effort. The existing daemon is also polled through `codex app-server proxy` and its documented WebSocket
  protocol, `initialize`, then metadata-only `thread/read`. Only loaded threads count: `notLoaded` is historical.
  This observes model/effort changes before another turn, including resumed threads. The same read-only CLI proxy
  runs over the existing SSH connection for a remote machine. Older CLIs with no proxy fall back to transcripts.
- **Codex unresolved defaults:** inspected CLI 0.163.0's generated `Thread` schema explicitly describes
  `reasoningEffort` as null when "unset or unavailable". A live 0.165.0 daemon returned a concrete model and null
  effort for default sessions; other live sessions returned concrete `low`. Actual October 29 turn contexts also
  omitted top-level effort and had null collaboration effort. `model/list.defaultReasoningEffort` is a suggested
  catalog default, not execution telemetry. We therefore show **Not reported**, never claim that value is the
  actual effort. The default effort is not fully observable through these interfaces.
- **Claude Code:** the main assistant response's concrete `message.model` (synthetic error models and sidechains
  excluded); hooks' `model` and `effort.level`. Real 2.1.310 Stop hook samples include `effort.level`. Hooks whitelist
  those fields and never copy credentials or full configuration. Local agents launched before the recorder upgrade
  need a restart to install the new recorder; remote raw hook events already retain these fields. A model change
  is visible on the next response or hook; a terminal picker change with neither observation remains unobserved.
- **Pi:** the main assistant response’s `message.model`, plus `model_change.modelId` and `thinking_level_change.thinkingLevel`, including `off`. Separate events merge
  within that session. These record CLI configuration rather than guessing from the chosen model.
  **Resume limitation:** Pi restores saved model/thinking state without appending initial settings events for an
  existing session (`createAgentSession`, upstream `sdk.ts`, initial-entry block guarded by `!hasExistingSession`).
  Old pre-launch events cannot prove that CLI/config overrides did not change it. After a resume, the model is
  observable on the next assistant response; effort remains **Not reported** until a new thinking-level event.
  A picker change without an event is likewise not observable. Full resumed-Pi effort support is not claimed.
  [Pi startup source](https://github.com/badlogic/pi-mono/blob/main/packages/coding-agent/src/core/sdk.ts).
- **Other Herdr agent kinds:** Herdr's `AgentInfo` reports kind, status and session identity/path, not model/effort.
  Their transcript formats are not implemented here: **Not reported**. Lack of a reader does not prove that effort
  is unsupported. `effortUnsupported` is reserved for a provider's explicit unsupported report; it renders
  **Not supported**, never an invented effort level. Codex’s live `model/list` can explicitly report an empty
  `supportedReasoningEfforts` list for the current model; that is used only as a capability report, never a default.

The existing daemon is only read. Failure, unavailable metadata and unsupported CLI versions leave values unreported.
No CLI configuration files, secrets or auth files are read. Cached daemon observations expire after ten seconds and
are discarded on restart. A daemon response for an earlier launch is ignored.

References: [Codex app-server protocol and metadata-only thread reads](https://learn.chatgpt.com/docs/app-server).
The per-version schemas can be inspected with `codex app-server generate-ts --out <temporary directory>`.
