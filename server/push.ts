// Push alerts to installed web apps (Web Push with this server's own VAPID keys).
import webpush from 'web-push'
import { pushPayload } from '../shared/alerts.ts'
import type { Alert } from '../shared/types.ts'
import { readJson, writeJson } from './store.ts'

type Sub = { sub: webpush.PushSubscription; device: string; at: number }

export class Push {
  readonly publicKey: string
  private subs: Sub[]
  constructor(contact = 'mailto:shepherdr@localhost') {
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
        webpush.sendNotification(s.sub, body, { TTL: 3600, urgency: 'high' }).catch((e: { statusCode?: number }) => {
          if (e.statusCode === 404 || e.statusCode === 410) gone.push(s.sub.endpoint)
        }),
      ),
    )
    if (gone.length) {
      this.subs = this.subs.filter((s) => !gone.includes(s.sub.endpoint))
      writeJson('push.json', this.subs)
    }
  }
}
