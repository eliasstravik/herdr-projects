// Push alerts as the push services see them: signed for a real address (Apple turns down a localhost one), a refusal
// kept and said rather than swallowed, a gone subscription forgotten.
import { afterAll, describe, expect, spyOn, test } from 'bun:test'
import { createECDH, randomBytes } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { Agent } from 'node:https'
import { join } from 'node:path'
import webpush from 'web-push'
import { Push, PUSH_CONTACT } from '../server/push.ts'
import { Settings } from '../server/live/settings.ts'
import type { Alert } from '../shared/types.ts'

// a push service of our own, over https as every real one (web-push sends only that, with its self-signed certificate
// trusted): answers each subscription's POST with the status (and body) set for it, and keeps what it was sent
// (subscriptions stay in push.json from one test to the next)
const cert = readFileSync(join(import.meta.dir, 'fixtures/push-service.crt'), 'utf8')
const agent = new Agent({ ca: cert })
const send = webpush.sendNotification.bind(webpush)
const spy = spyOn(webpush, 'sendNotification').mockImplementation((sub, payload, options) => send(sub, payload, { ...options, agent }))
const answers: Record<string, { status: number; body?: string }> = {}
const seen: { auth: string }[] = []
const service = Bun.serve({
  port: 0,
  hostname: '127.0.0.1',
  tls: { cert, key: readFileSync(join(import.meta.dir, 'fixtures/push-service.key'), 'utf8') },
  fetch(req) {
    seen.push({ auth: req.headers.get('authorization') ?? '' })
    const a = answers[new URL(req.url).pathname.slice(1)] ?? { status: 201 }
    return new Response(a.body ?? '', { status: a.status })
  },
})
afterAll(() => {
  service.stop(true)
  spy.mockRestore()
})

const b64 = (b: Buffer) => b.toString('base64url')
const subscription = (name: string) => {
  const ecdh = createECDH('prime256v1')
  ecdh.generateKeys()
  return { endpoint: `https://127.0.0.1:${service.port}/${name}`, keys: { p256dh: b64(ecdh.getPublicKey()), auth: b64(randomBytes(16)) } }
}
const alert: Alert = { id: 'a1', from: 'Shepherdr', title: 'X-1 needs you', body: '', go: { place: 'all', task: 'X-1' } }
/** the VAPID JWT's claims from an Authorization header (vapid t=<jwt>, k=<key>) */
const claims = (auth: string) => JSON.parse(Buffer.from(/t=([^,]+)/.exec(auth)![1]!.split('.')[1]!, 'base64url').toString()) as { sub: string; aud: string }

describe('push alerts', () => {
  test('signed for a real https address, never localhost (Apple answers 403 BadJwtToken to that)', async () => {
    expect(PUSH_CONTACT).toStartWith('https://')
    const push = new Push()
    push.subscribe(subscription('signed'), 'iPhone')
    await push.send(alert)
    const { sub } = claims(seen.at(-1)!.auth)
    expect(sub).toBe(PUSH_CONTACT)
    expect(sub).not.toContain('localhost')
  })

  test('a refusal is kept and logged until an alert to that device goes through', async () => {
    const warn = spyOn(console, 'warn').mockImplementation(() => {})
    const push = new Push()
    push.subscribe(subscription('refused'), 'Mozilla/5.0 (iPhone)')
    answers.refused = { status: 403, body: '{"reason":"BadJwtToken"}' }
    await push.send(alert)
    expect(push.failure).toMatchObject({ device: 'Mozilla/5.0 (iPhone)', reason: '403 BadJwtToken' })
    expect(String(warn.mock.calls.at(-1)?.[0])).toContain('403 BadJwtToken')
    // kept: a refusal is no reason to forget the device
    expect(push.devices).toContain('Mozilla/5.0 (iPhone)')
    answers.refused = { status: 201 }
    await push.send(alert)
    expect(push.failure).toBeNull()
    // turned down, then gone: nothing failing is said for a device no longer there
    answers.refused = { status: 403, body: 'Forbidden' }
    await push.send(alert)
    expect(push.failure?.reason).toBe('403 Forbidden')
    answers.refused = { status: 410 }
    await push.send(alert)
    expect(push.failure).toBeNull()
    warn.mockRestore()
  })

  test('a gone subscription is forgotten, with no failure', async () => {
    const push = new Push()
    push.subscribe(subscription('gone'), 'old Mac')
    answers.gone = { status: 410 }
    await push.send(alert)
    expect(push.devices).not.toContain('old Mac')
    expect(push.failure).toBeNull()
  })

  test('Settings → Alerts on says the last failure under its devices', () => {
    const row = (failed?: string) =>
      new Settings().sections(['a phone'], [], null, 'this machine', [], 'this machine', undefined, failed).flatMap((s) => s.rows).find((r) => r.key === 'alerts')!
    expect(row().warn).toBeUndefined()
    expect(row('Last alert to a phone failed 5m ago: 403 BadJwtToken')).toMatchObject({ value: 'a phone', warn: 'Last alert to a phone failed 5m ago: 403 BadJwtToken' })
  })
})
