# Agent secret access

`secret run` and `secret env` require a fresh per-agent 256-bit capability. General CLI access, loopback, Host,
forwarding headers and `helper/main.sock` are not authentication. The HTTP adapter reads a bearer Authorization
header; it never accepts proof in the request body. Only the worker that issued a capability can validate it.

The worker provisions it only after finding the actual agent CLI process through herdr. Its private in-memory
record binds it to that process launch, exact agent/coordinator/reviewer chat, and a fixed chain: that agent's space,
its owning machine, Everywhere. Retrieval ignores the request's chat, cwd and flags when choosing levels. The current
server-owned scope must still match. Home/main coordinators receive their home-space chain, not every space or
machine. A child reviewer can use its parent's credential for the same space; a separately launched reviewer gets
its own credential. Remote workers have separate authorities and their own machine chains, even though the vault
is stored on the main machine.

`agentEnv` passes only an opaque `SHEPHERDR_SECRET_FILE` reference to the launch environment (including Codex's shell
policy). The capability is written via stdin over the existing private SSH connection, never in launch arguments,
agent records, prompt text or CLI output. `helper/route.pl --bind PID` reads JSON on stdin, creates owner-only files
in owner-only `~/.shepherdr/helper/routes` and `credentials` directories, and binds them to the process start stamp.
The CLI obtains proof from a live ancestor's binding or that explicit credential reference. Folder/chat routing may
choose the command's destination, but cannot grant a credential. The helper carries Authorization unchanged through
main.sock and the SSH forward; there is no `local=true` bypass. The transient bearer environment variable is removed
before running the user's command. Private Shepherdr state cannot be imported into browser-visible artifacts, even
through symlinks. Snapshots, names, list output and secret cards contain no values.

Capabilities rotate on a new launch or changed scope, are revoked on finish/forget/relaunch and observed agent loss,
and all expire on worker restart. Revoked files are inert; stale files also fail their live-process check. Route repair
reprovisions running agents without restarting them. Credentials deliberately are not persisted in the server's
records. Owner-only files protect against other OS users, not hostile code already running as the same OS user:
that code can read the vault itself. Use separate OS accounts/machines when agents must be mutually untrusted.

## Upgrade and compatibility

Deploy the updated server and helper together. Until provisioning succeeds, secret retrieval fails closed. No
unauthenticated terminal or legacy helper fallback exists. Ordinary CLI commands and secret names/input keep their
syntax. Human terminals without an agent credential can still manage names and values through Settings and existing
set/rm commands; secret run/env must run from an authenticated agent session.

Existing agents with their own process ancestry are repaired automatically on the next route refresh (about 30 s).
Older shared tool executors cannot be authenticated by chat/cwd alone. They must carry the session's provisioned
`SHEPHERDR_SECRET_FILE` from its launch environment; if it is missing, a planned agent restart is required to inherit
it. Do not copy another session's credential, print its contents, or restart a working agent without approval. A
custom `--url` target deliberately drops inherited credentials; it must provision its own authenticated session.

Only `bun run typecheck` runs on agent machines. Regression suites use temporary fake vaults, homes, credentials,
loopback servers and Unix sockets; GitHub runs them after merge. They cover no/invalid/revoked proof, raw TCP
forwarding, spoofed headers, scope forgery, rotation, worker isolation and repo/helper secret-run transport.
