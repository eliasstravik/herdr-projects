// Push alerts to installed web apps (Web Push with this server's own VAPID keys).
import webpush from 'web-push'
import { pushPayload } from '../shared/alerts.ts'
import type { Alert } from '../shared/types.ts'
import { readJson, writeJson } from './store.ts'

type Sub = { sub: webpush.PushSubscription; device: string; at: number }
/** the last alert a push service turned down: when, the device, and its answer ("403 BadJwtToken") */
export type PushFailure = { at: number; device: string; reason: string; endpoint: string }

/** The VAPID subject: who runs this sender, for the push services. Apple's turns down every alert signed for a
 *  localhost one (403 BadJwtToken), so it is a real https address unless SHEPHERDR_PUSH_CONTACT names another
 *  (https://… or mailto:…). */
export const PUSH_CONTACT = process.env.SHEPHERDR_PUSH_CONTACT || 'https://github.com/eliasstravik/shepherdr'

export class Push {
  readonly publicKey: string
  private subs: Sub[]
  /** the last failure, kept until an alert to that device goes through again */
  failure: PushFailure | null = null
  constructor(contact = PUSH_CONTACT) {
    let keys = readJson<{ publicKey: string; privateKey: string } | null>('vapid.json', null)
    if (!keys) {
      keys = webpush.generateVAPIDKeys()
      writeJson('vapid.json', keys)
    }
    this.publicKey = keys.publicKey
    webpush.setVapidDetails(contact, keys.publicKey, keys.privateKey)
    this.subs = readJson<Sub[]>('push.json', [])
  }
  get devices() {
    return this.subs.map((s) => s.device)
  }
  subscribe(sub: webpush.PushSubscription, device: string) {
    this.subs = this.subs.filter((s) => s.sub.endpoint !== sub.endpoint)
    this.subs.push({ sub, device, at: Date.now() })
    writeJson('push.json', this.subs)
  }
  /** skip: subscriptions whose app is on screen right now; waits: what waits on you now (the icon's count, and what
   *  older alerts may stay) */
  async send(a: Alert, skip: Set<string> = new Set(), waits: { count: number; tags: string[] } = { count: 0, tags: [] }) {
    const body = JSON.stringify(pushPayload(a, waits))
    const gone: string[] = []
    const to = this.subs.filter((s) => !skip.has(s.sub.endpoint))
    await Promise.all(
      to.map((s) =>
        webpush.sendNotification(s.sub, body, { TTL: 3600, urgency: 'high' }).then(
          () => {
            if (this.failure?.endpoint === s.sub.endpoint) this.failure = null
          },
          (e: { statusCode?: number; body?: string; message?: string }) => {
            // unsubscribed or gone: forgotten; anything else is said, never swallowed
            if (e.statusCode === 404 || e.statusCode === 410) return gone.push(s.sub.endpoint)
            this.failure = { at: Date.now(), device: s.device, reason: failureWords(e), endpoint: s.sub.endpoint }
            console.warn(`push to ${URL.parse(s.sub.endpoint)?.host ?? 'a push service'} (${s.device}) failed: ${this.failure.reason}`)
          },
        ),
      ),
    )
    if (gone.length) {
      this.subs = this.subs.filter((s) => !gone.includes(s.sub.endpoint))
      writeJson('push.json', this.subs)
    }
  }
}

/** "403 BadJwtToken": the status and the push service's reason, or what went wrong before an answer */
function failureWords(e: { statusCode?: number; body?: string; message?: string }) {
  if (!e.statusCode) return e.message || 'no answer'
  let reason = (e.body ?? '').trim()
  try {
    reason = String((JSON.parse(reason) as { reason?: string }).reason ?? reason)
  } catch {
    /* plain text */
  }
  return `${e.statusCode}${reason ? ` ${reason.slice(0, 120)}` : ''}`
}
