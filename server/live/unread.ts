// Unread, as Slack has it: when each chat last had something new from its agent or Shepherdr, and how far you have read
// each one, so the sidebar and the tasks list show where new messages came in. Each machine keeps the first for its own
// chats; the main machine keeps the read marks for all of them, so every device shows the same.
import { readJson, writeJson } from '../store.ts'

interface Kept {
  last: Record<string, number>
  read: Record<string, number>
}

const FILE = 'unread.json'

export class Unread {
  private d: Kept

  constructor() {
    this.d = readJson<Kept>(FILE, { last: {}, read: {} })
    // what came before read marks were kept counts as read
    if (!this.d.read['*']) {
      this.d.read['*'] = Date.now()
      this.save()
    }
  }

  private save() {
    writeJson(FILE, this.d)
  }

  get last(): Record<string, number> {
    return this.d.last
  }

  get read(): Record<string, number> {
    return this.d.read
  }

  /** the chat's newest thing from its agent or Shepherdr is from then: true when that is newer than it was */
  saw(key: string, at: number | undefined) {
    if (!at || at <= (this.d.last[key] ?? 0)) return false
    this.d.last[key] = at
    this.save()
    return true
  }

  /** you have read these chats, each up to then; true when any moved on */
  mark(chats: Record<string, number>) {
    let moved = false
    for (const [key, at] of Object.entries(chats)) {
      if (key === '*' || typeof at !== 'number' || !Number.isFinite(at) || at <= (this.d.read[key] ?? 0)) continue
      this.d.read[key] = at
      moved = true
    }
    if (moved) this.save()
    return moved
  }
}
