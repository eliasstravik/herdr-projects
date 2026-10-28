// The shepherdr command: sends its words to the Shepherdr server on this machine and prints the answer.
// Inside an agent's terminal, SHEPHERDR_CHAT says which agent is asking.
import { readJson } from './store.ts'

const args = process.argv.slice(2)
const url = process.env.SHEPHERDR_URL ?? `http://127.0.0.1:${readJson<{ port?: number }>('server.json', {}).port ?? process.env.SHEPHERDR_PORT ?? 7787}`
// the words that take a text on stdin: a secret, a new HEARTBEAT.md
const stdin = (args[0] === 'secret' || (args[0] === 'heartbeat' && args[1] === 'set')) && !process.stdin.isTTY ? await Bun.stdin.text() : undefined

// shepherdr merge answers within 90 s (MERGE_DEADLINE, engine.ts): no answer a little after that means none is coming
// (Shepherdr restarted, or its link dropped), never a wait on and on
const merge = args[0] === 'merge' || (args[0] === 'task' && args[1] === 'merge')
const wait = merge ? Number(process.env.SHEPHERDR_MERGE_WAIT ?? 110_000) : 0

try {
  const r = await fetch(`${url}/api/cli`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ args, chat: process.env.SHEPHERDR_CHAT, cwd: process.cwd(), stdin }),
    signal: wait ? AbortSignal.timeout(wait) : undefined,
  })
  const j = (await r.json()) as { out: string; code: number }
  process.stdout.write(j.out)
  process.exit(j.code)
} catch (e) {
  if (merge && (e as Error).name === 'TimeoutError') {
    process.stderr.write(noAnswer(wait, args[0] === 'merge' ? args[1] : args[2]))
    process.exit(1)
  }
  process.stderr.write(`Shepherdr is not running here (${url}). Start it from herdr's plugin actions, or: bun ${import.meta.dir}/main.ts\n`)
  process.exit(2)
}

/** what to do when shepherdr merge heard nothing back (the helper's shepherdr says the same) */
function noAnswer(ms: number, id = '<id>') {
  return `shepherdr merge ${id} got no answer from Shepherdr in ${Math.round(ms / 1000)} s (it may have restarted). Don't leave it running: see where it is with shepherdr task show ${id}, and unless it says Merged, run shepherdr merge ${id} again; it finishes the merge, or says it is merged.\n`
}
