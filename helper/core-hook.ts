#!/usr/bin/env bun
// Per-launch hooks for core agents. No HTTP, caller-selected chat authority, or screen scraping.
import { appendFileSync } from 'node:fs'
import { randomUUID } from 'node:crypto'
import { readCredential } from '../server/core/home.ts'
import { coreRequest } from '../server/core/request.ts'
import type { CoreEndpoint } from '../server/core/client.ts'
import type { MailCredential } from '../server/core/envelope.ts'
import type { AgentHook } from '../server/core/model.ts'
import type { PermissionAnswer } from '../server/core/native-asks.ts'

const credentialFile = process.env.SHEPHERDR_CORE_CREDENTIAL
const hookFile = process.env.SHEPHERDR_CORE_HOOK_FILE
let permission = false
try {
  const raw = await Bun.stdin.text()
  if (Buffer.byteLength(raw) > 1024 * 1024) throw new Error('Hook input too large')
  const input = JSON.parse(raw)
  permission = input.hook_event_name === 'PermissionRequest'
  if (!credentialFile || !hookFile || typeof input.hook_event_name !== 'string') throw new Error('Missing launch identity')
  const credential = readCredential<{ mail: MailCredential; socket?: string; endpoint?: CoreEndpoint }>(credentialFile)
  const cut = (value: unknown, max: number) => typeof value === 'string' ? value.slice(0, max) : undefined
  const event: AgentHook = {
    id: randomUUID(), event: input.hook_event_name, at: Date.now(), session: cut(input.session_id, 128),
    transcript: cut(input.transcript_path, 4096), agent: cut(input.agent_id, 128), model: cut(input.model, 128), tool: cut(input.tool_name, 128), mode: cut(input.permission_mode, 32),
  }
  appendFileSync(hookFile, JSON.stringify(event) + '\n', { mode: 0o600 })
  if (permission) {
    const answer = await coreRequest(credential.endpoint ?? credential.socket ?? (() => { throw new Error('Missing core endpoint') })(), credential.mail, 'permission', {
      id: event.id, tool: event.tool, input: input.tool_input, session: event.session, agent: event.agent, at: event.at,
    }, { id: event.id, timeout: 3_500_000 }) as PermissionAnswer
    const { allow, ...reply } = answer
    console.log(JSON.stringify({ hookSpecificOutput: { hookEventName: 'PermissionRequest', decision: {
      behavior: allow === true ? 'allow' : 'deny', ...(allow === true ? {} : { message: 'The human declined this request.' }), ...reply,
    } } }))
  }
} catch {
  if (permission) console.log(JSON.stringify({ hookSpecificOutput: { hookEventName: 'PermissionRequest', decision: { behavior: 'deny', message: 'Shepherdr could not verify approval. Ask again after it reconnects.' } } }))
  // Ordinary observation hooks never block the agent. Permission transport failures always deny.
}
