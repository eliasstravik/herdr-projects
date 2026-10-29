// Workers run on the main machine, with private records under one installation's HOME.
// This durable mailbox needs neither a live worker link nor a second overview/news emission.
import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { randomUUID } from 'node:crypto'

export interface RequestedNews { id: string; to: string; from: string; text: string; task: string; event: string }
export class RequestedNewsMailbox {
  private dir: string
  private sequence = 0
  constructor(private root: string) { this.dir = join(root, 'requested-news') }
  /** Only a real coordinator in this installation, never a display name, task agent or arbitrary chat. */
  known(chat?: string): string | undefined {
    if (!chat || !/^p:[^:]+:[^/]+$/.test(chat)) return
    const homes = [this.root]
    const machines = join(this.root, 'machines')
    if (existsSync(machines)) for (const name of readdirSync(machines)) homes.push(join(machines, name))
    for (const home of homes) {
      try {
        const records = JSON.parse(readFileSync(join(home, 'agents.json'), 'utf8'))
        if (records[chat]?.key === chat && !records[chat].finished) return chat
      } catch { /* no coordinator records here */ }
    }
  }
  post(news: Omit<RequestedNews, 'id'>) {
    mkdirSync(this.dir, { recursive: true, mode: 0o700 })
    const item = { ...news, id: `${Date.now()}-${String(this.sequence++).padStart(8, '0')}-${randomUUID()}` }
    const file = join(this.dir, `${item.id}.json`)
    writeFileSync(`${file}.tmp`, JSON.stringify(item), { mode: 0o600 })
    renameSync(`${file}.tmp`, file)
  }
  async drain(owns: (chat: string) => boolean, deliver: (n: RequestedNews) => Promise<void>) {
    if (!existsSync(this.dir)) return
    for (const name of readdirSync(this.dir).filter((n) => /^[a-f0-9-]+\.json$/.test(n)).sort()) {
      const file = join(this.dir, name)
      let n: RequestedNews
      try { n = JSON.parse(readFileSync(file, 'utf8')) } catch { continue }
      if (!owns(n.to)) continue
      await deliver(n)
      rmSync(file, { force: true })
    }
  }
}
