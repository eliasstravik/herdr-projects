// The tasks engine's test world, shared by the engine*.test.ts files: a real git repo, the agents stubbed, herdr a
// stand-in that makes and removes real git worktrees. Each file that imports it gets its own Shepherdr home.
import { execSync } from 'node:child_process'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

process.env.SHEPHERDR_HOME = mkdtempSync(join(tmpdir(), 'shep-engine-'))
export const { Engine } = await import('../server/live/engine/engine.ts')
export const { Places, placeId } = await import('../server/live/places.ts')
export const { Settings } = await import('../server/live/settings.ts')
export const { settingsBlock } = await import('../server/live/prompts.ts')
export const { writeJson } = await import('../server/store.ts')
export const { fakeHerdr } = await import('./fake-herdr.ts')
export const G = await import('../server/live/engine/git.ts')
export type Said = import('../server/live/readers/hooks.ts').Said

export const sh = (cmd: string, cwd: string) => execSync(cmd, { cwd, shell: '/bin/sh', encoding: 'utf8' }).trim()

export function repo() {
  const dir = mkdtempSync(join(tmpdir(), 'shep-repo-'))
  sh('git init -q -b main && git config user.email t@t && git config user.name T && echo hi > README.md && git add -A && git commit -qm init', dir)
  return dir
}

/** boot: what herdr says once the agent is ready; any but working leaves its brief waiting to go in */
export function world(herdr = fakeHerdr(), o: { boot?: string } = {}) {
  const started: { key: string; first?: string; cwd: string; system?: string; pane?: string }[] = []
  const status: Record<string, string> = {}
  const sent: { key: string; text: string }[] = []
  const records: Record<string, { kind: string; started: number; finished?: boolean }> = {}
  const panes: Record<string, string> = {}
  const outbox: string[] = []
  /** work an agent started in the background that still runs: a subagent, or shells */
  const bg: Record<string, string | undefined> = {}
  /** what an agent reports of itself through its hooks */
  const said: Record<string, Said | undefined> = {}
  const agents = {
    ensure: async (s: { key: string; first?: string; cwd: string; kind: string; pane?: () => Promise<string> }) => {
      const pane = await s.pane?.()
      if (pane) {
        panes[s.key] = pane
        herdr.agentPanes.add(pane)
      }
      started.push({ ...s, pane })
      records[s.key] = { kind: s.kind, started: Date.now() }
      status[s.key] = o.boot ?? 'working'
      if (o.boot) outbox.push(s.key)
      return null
    },
    send: async (key: string, text: string) => {
      sent.push({ key, text })
      return { waiting: false, item: {} }
    },
    status: (key: string) => status[key] ?? 'gone',
    background: (key: string) => bg[key],
    said: (key: string) => said[key],
    record: (key: string) => records[key],
    info: (key: string) => (status[key] && status[key] !== 'gone' && panes[key] ? { pane_id: panes[key] } : undefined),
    waiting: (key: string) => outbox.filter((k) => k === key),
    finish: async (key: string) => {
      if (records[key]) records[key].finished = true
      status[key] = 'gone'
      if (panes[key]) await herdr.h.closePane(panes[key])
    },
    interrupt: async () => undefined,
    stopTurn: async (key: string) => void (status[key] === 'working' && stopped.push(key)),
    restart: async (s: { key: string; first?: string; cwd: string; kind: string; pane?: () => Promise<string> }, before?: () => Promise<string | void>) => {
      const first = await before?.()
      await agents.finish(s.key)
      return agents.ensure({ ...s, ...(first ? { first } : {}) })
    },
    forget: (key: string) => {
      delete records[key]
      forgot.push(key)
    },
  }
  const forgot: string[] = []
  /** agents whose turn was interrupted before a restart */
  const stopped: string[] = []
  const posted: { key: string; id: string; question?: { kind: string; steps?: string[] }; card?: { text: string; restart?: boolean } }[] = []
  /** what the agent's chat holds: its last reply, an open card of Shepherdr's, its CLI's own question */
  const chat: { reply?: string; card?: 'plan' | 'ask'; cardTitle?: string; question?: boolean; unseen?: string[]; messages?: { who: string; text?: string; card?: object }[] } = { reply: 'Which name do you want?' }
  const chats = {
    post: (key: string, p: { id: string; question?: { kind: string } }) => posted.push({ key, ...p }),
    lastReply: () => chat.reply,
    openQuestion: () => (chat.question ? { id: 'q', items: [{ question: 'Which one?', options: [], multiSelect: false }] } : undefined),
    openCard: () => chat.card,
    openCardTitle: () => (chat.card ? chat.cardTitle : undefined),
    unseen: () => chat.unseen ?? [],
    menu: (key: string) => (status[key] === 'blocked' ? { title: 'Allow: rm -rf build' } : undefined),
    forget: (key: string) => forgot.push(`chat ${key}`),
    chat: () => ({ messages: chat.messages ?? [] }),
  }
  const places = new Places()
  const settings = new Settings()
  const engine = new Engine(agents as never, chats as never, places, settings, herdr.h as never)
  const news: string[] = []
  /** the place each news is for: a project's place id, as its coordinator's chat has it */
  const newsTo: string[] = []
  engine.on('news', (p, text) => (newsTo.push(p), news.push(text)))
  const cards: { task: string; status: string; sub: string }[] = []
  engine.on('card', (t, status, sub) => cards.push({ task: t.id, status, sub }))
  /** herdr says the agent's state changed */
  const agentIs = (key: string, st: string) => {
    status[key] = st
    engine.agentStatus(key)
  }
  /** what waits to go in to it goes in */
  const brief = (key: string) => {
    outbox.splice(0, outbox.length, ...outbox.filter((k) => k !== key))
    engine.sent(key)
  }
  return { engine, places, settings, started, sent, posted, status, records, news, newsTo, cards, agentIs, brief, herdr, chat, bg, forgot, said, stopped }
}

export const until = async (f: () => boolean, ms = 8000) => {
  const t0 = Date.now()
  while (!f()) {
    if (Date.now() - t0 > ms) throw new Error('timed out')
    await Bun.sleep(20)
  }
}

/** a task whose agent has started in its worktree */
export async function started(w: ReturnType<typeof world>, dir: string, name: string, title = 'Say hello', desc = 'Add hello.txt. Merge when done.') {
  const p = await w.places.add(dir, name)
  const t = w.engine.add({ project: p.name, title, desc, by: 'test' })
  await until(() => w.started.some((s) => s.key === `${t.id}/Agent`))
  return { p, t, key: `${t.id}/Agent` }
}
