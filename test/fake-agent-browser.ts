#!/usr/bin/env bun
// agent-browser, standing in for the real one. Run as a command: the commands Shepherdr runs, answered from a state
// file the test keeps (FAKE_AGENT_BROWSER: a folder with state.json; every call is logged to calls.log with the
// session it was for). Imported: fakeAgentBrowser() starts its stream and DevTools servers and keeps that folder.
import { appendFileSync, chmodSync, existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import type { ServerWebSocket } from 'bun'

/** what the page says about any element tapped */
export const PICKED = { element: 'button "Save"', selector: 'form > button.primary', text: '', box: { x: 10, y: 20, w: 100, h: 30 }, at: { x: 50, y: 435 }, fixed: false, frame: false }

/** the stand-in's stream and DevTools servers, and the folder its command answers from */
export function fakeAgentBrowser() {
  const bin = join(import.meta.dir, 'fake-agent-browser.ts')
  chmodSync(bin, 0o755)
  const dir = mkdtempSync(join(tmpdir(), 'shep-ab-'))
  const streamSocks = new Set<ServerWebSocket<unknown>>()
  const fromPanel: Record<string, unknown>[] = []
  const stream = Bun.serve({
    port: 0,
    hostname: '127.0.0.1',
    fetch: (req, s) => (s.upgrade(req) ? undefined : new Response('no')),
    websocket: {
      open(ws) {
        streamSocks.add(ws)
        ws.send(JSON.stringify({ type: 'status', connected: true, screencasting: true, viewportWidth: 1280, viewportHeight: 720 }))
        ws.send(JSON.stringify({ type: 'tabs', tabs: [{ active: true, targetId: 'T1', title: 'Settings', url: 'http://localhost:5173/settings' }] }))
      },
      message(_, m) {
        fromPanel.push(JSON.parse(String(m)))
      },
      close(ws) {
        streamSocks.delete(ws)
      },
    },
  })
  const toPanels = (m: unknown) => streamSocks.forEach((s) => s.send(JSON.stringify(m)))

  const cdpCalls: { method: string; params: Record<string, unknown>; sessionId?: string }[] = []
  const cdp = Bun.serve({
    port: 0,
    hostname: '127.0.0.1',
    fetch: (req, s) => (s.upgrade(req) ? undefined : new Response('no')),
    websocket: {
      message(ws, raw) {
        const m = JSON.parse(String(raw)) as { id: number; method: string; params: Record<string, unknown>; sessionId?: string }
        cdpCalls.push(m)
        const result: Record<string, unknown> =
          m.method === 'Target.attachToTarget'
            ? { sessionId: 'S1' }
            : m.method === 'Runtime.evaluate'
              ? { result: { value: [0, 400] } }
              : m.method === 'DOM.getNodeForLocation'
                ? { backendNodeId: 7 }
                : m.method === 'DOM.resolveNode'
                  ? { object: { objectId: 'o7' } }
                  : m.method === 'Runtime.callFunctionOn'
                    ? { result: { value: String(m.params.functionDeclaration).includes('selector') ? PICKED : { x: 1, y: 2, w: 3, h: 4 } } }
                    : m.method === 'Page.captureScreenshot'
                      ? { data: Buffer.from('png').toString('base64') }
                      : m.method === 'Page.getNavigationHistory'
                        ? { currentIndex: 1, entries: [{ id: 11 }, { id: 12 }] }
                        : {}
        ws.send(JSON.stringify({ id: m.id, result, sessionId: m.sessionId }))
      },
    },
  })

  const state = (s: Record<string, unknown> = {}) => writeFileSync(join(dir, 'state.json'), JSON.stringify({ connected: false, port: stream.port, cdp: `ws://127.0.0.1:${cdp.port}/devtools/browser/x`, sessions: [], ...s }))
  const calls = () =>
    existsSync(join(dir, 'calls.log'))
      ? readFileSync(join(dir, 'calls.log'), 'utf8')
          .trim()
          .split('\n')
          .filter(Boolean)
          .map((l) => JSON.parse(l) as { args: string[]; session: string; namespace: string; profile: string })
          .filter((c) => c.args[0] !== '--version')
      : []

  const reset = () => {
    state()
    writeFileSync(join(dir, 'calls.log'), '')
    fromPanel.length = 0
    cdpCalls.length = 0
  }
  reset()
  const stop = () => {
    stream.stop(true)
    cdp.stop(true)
  }
  return { bin, dir, stream, cdp, streamSocks, fromPanel, toPanels, cdpCalls, state, calls, reset, stop, env: { FAKE_AGENT_BROWSER: dir } }
}

if (import.meta.main) cli()

function cli() {
  if (process.argv[2] === '--version') return console.log('agent-browser 0.39.2')
  const dir = process.env.FAKE_AGENT_BROWSER!
  const file = join(dir, 'state.json')
  const state = JSON.parse(readFileSync(file, 'utf8')) as { connected: boolean; port: number; cdp: string; sessions: string[]; fail?: string; clash?: number }
  const args = process.argv.slice(2).filter((a) => a !== '--json')
  const session = process.env.AGENT_BROWSER_SESSION ?? 'default'
  appendFileSync(join(dir, 'calls.log'), `${JSON.stringify({ args, session, namespace: process.env.AGENT_BROWSER_NAMESPACE, profile: process.env.AGENT_BROWSER_PROFILE })}\n`)
  const save = () => writeFileSync(file, JSON.stringify(state))
  const out = (data: unknown, error?: string) => console.log(JSON.stringify({ success: !error, data: error ? null : data, error: error ?? null }))

  const [a, b] = args
  // another agent-browser started the session's daemon meanwhile, this many more times
  if (state.clash) {
    state.clash--
    save()
    return out(null, `A daemon for session '${session}' started concurrently with different daemon configuration. Retry the command so agent-browser can restart it with the requested configuration.`)
  }
  if (a === '--version') console.log('agent-browser 0.39.2')
  else if (a === 'stream' && b === 'status') out({ connected: state.connected, enabled: true, port: state.port })
  else if (a === 'open') {
    if (state.fail) out(null, state.fail)
    else {
      state.connected = true
      if (!state.sessions.includes(session)) state.sessions.push(session)
      save()
      out({ url: args[1] })
    }
  } else if (a === 'get' && b === 'cdp-url') out({ cdpUrl: state.cdp })
  else if (a === 'tab' && b === 'list') out({ tabs: [{ active: true, tabId: 't1', targetId: 'T1', url: 'about:blank' }] })
  else if (a === 'session' && b === 'list') out({ sessions: state.sessions })
  else if (a === 'close') {
    state.sessions = state.sessions.filter((s) => s !== session)
    state.connected = false
    save()
    out({ closed: true })
  } else if (a === 'set' && b === 'viewport') out({ width: Number(args[2]), height: Number(args[3]) })
  else out(null, `unknown command: ${args.join(' ')}`)
}
