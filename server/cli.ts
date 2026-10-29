// The shepherdr command: sends its words to the Shepherdr server on this machine and prints the answer.
// Inside an agent's terminal, SHEPHERDR_CHAT says which agent is asking.
import { spawnSync } from 'node:child_process'
import { chmodSync, existsSync, lstatSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { readJson } from './store.ts'

/** values shorter than this aren't masked: they would garble everything else */
const MASKED_MIN = 4
const MASK = '********'

const all = process.argv.slice(2)
const url = process.env.SHEPHERDR_URL ?? `http://127.0.0.1:${readJson<{ port?: number }>('server.json', {}).port ?? process.env.SHEPHERDR_PORT ?? 7787}`
// shepherdr secret run NAME … -- <command>: only the names go to Shepherdr; the command runs here, with the values in
// its environment. shepherdr secret env writes them to a file here. Neither prints a value
const secret = all[0] === 'secret' ? all[1] : undefined
const cut = all.indexOf('--')
const command = secret === 'run' && cut > 0 ? all.slice(cut + 1) : []
const args = secret === 'run' && cut > 0 ? all.slice(0, cut) : all
if (secret === 'run' && !command.length) {
  process.stderr.write('Put the command after --: shepherdr secret run NAME [NAME …] -- <command> [its words]\n')
  process.exit(1)
}
// the words that take a text on stdin: a secret to keep, a new HEARTBEAT.md (never secret run's: that is its command's)
const stdin = ((secret === 'set' || secret === 'elevenlabs') || (args[0] === 'heartbeat' && args[1] === 'set')) && !process.stdin.isTTY ? await Bun.stdin.text() : undefined

// shepherdr merge answers within 90 s (MERGE_DEADLINE, engine.ts): no answer a little after that means none is coming
// (Shepherdr restarted, or its link dropped), never a wait on and on
const merge = args[0] === 'merge' || (args[0] === 'task' && args[1] === 'merge')
const wait = merge ? Number(process.env.SHEPHERDR_MERGE_WAIT ?? 110_000) : 0

let j: { out: string; code: number; secrets?: Record<string, string> }
try {
  const r = await fetch(`${url}/api/cli`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ args, chat: process.env.SHEPHERDR_CHAT, cwd: process.cwd(), stdin }),
    signal: wait ? AbortSignal.timeout(wait) : undefined,
  })
  j = (await r.json()) as typeof j
} catch (e) {
  if (merge && (e as Error).name === 'TimeoutError') {
    process.stderr.write(noAnswer(wait, args[0] === 'merge' ? args[1] : args[2]))
    process.exit(1)
  }
  process.stderr.write(`Shepherdr is not running here (${url}). Start it from herdr's plugin actions, or: bun ${import.meta.dir}/main.ts\n`)
  process.exit(2)
}
if (j.secrets && secret === 'run') process.exit(await run(command, j.secrets))
if (j.secrets && secret === 'env') {
  const f = envFile(args, j.secrets)
  process.stdout.write(f.out)
  process.exit(f.code)
}
process.stdout.write(j.out)
process.exit(j.code)

/** what to do when shepherdr merge heard nothing back (the helper's shepherdr says the same) */
function noAnswer(ms: number, id = '<id>') {
  return `shepherdr merge ${id} got no answer from Shepherdr in ${Math.round(ms / 1000)} s (it may have restarted). Don't leave it running: see where it is with shepherdr task show ${id}, and unless it says Merged, run shepherdr merge ${id} again; it finishes the merge, or says it is merged.\n`
}

/** shepherdr secret run: the command, with the values in its environment; what it prints comes through with each value
 *  masked, as 1Password's op run does, so one a command prints by mistake still lands in no terminal or chat */
async function run(command: string[], secrets: Record<string, string>) {
  let c: Bun.Subprocess<'inherit', 'pipe', 'pipe'>
  try {
    c = Bun.spawn(command, { stdin: 'inherit', stdout: 'pipe', stderr: 'pipe', env: { ...process.env, ...secrets } })
  } catch (e) {
    process.stderr.write(`Couldn't run ${command[0]}: ${(e as Error).message}\n`)
    return 127
  }
  const values = Object.values(secrets).filter((v) => v.length >= MASKED_MIN)
  await Promise.all([masked(c.stdout, process.stdout, values), masked(c.stderr, process.stderr, values)])
  const code = await c.exited
  return c.signalCode ? 128 + (({ SIGHUP: 1, SIGINT: 2, SIGQUIT: 3, SIGKILL: 9, SIGTERM: 15 }) as Record<string, number>)[c.signalCode] || 1 : code
}

/** a stream as it comes, each value masked; what may be the start of one waits for what follows */
async function masked(from: ReadableStream<Uint8Array>, to: NodeJS.WriteStream, values: string[]) {
  const dec = new TextDecoder()
  const all = [...values].sort((a, b) => b.length - a.length)
  const mask = (s: string) => all.reduce((x, v) => x.split(v).join(MASK), s)
  // how much of the end could begin a value
  const held = (s: string) => {
    for (let k = Math.min((all[0]?.length ?? 1) - 1, s.length); k > 0; k--) if (all.some((v) => v.length > k && s.endsWith(v.slice(0, k)))) return k
    return 0
  }
  const reader = from.getReader()
  let buf = ''
  for (let r = await reader.read(); !r.done; r = await reader.read()) {
    buf = mask(buf + dec.decode(r.value, { stream: true }))
    const keep = held(buf)
    to.write(buf.slice(0, buf.length - keep))
    buf = buf.slice(buf.length - keep)
  }
  to.write(mask(buf + dec.decode()))
}

/** shepherdr secret env: each NAME=value in the file (--file, .env by default), in place of its old line, the file
 *  readable by you only; refused where git would keep it (the helper's shepherdr does the same) */
function envFile(args: string[], secrets: Record<string, string>) {
  const i = args.findIndex((a) => a === '--file' || a.startsWith('--file='))
  const name = i < 0 ? '.env' : args[i].startsWith('--file=') ? args[i].slice(7) : args[i + 1]
  if (!name) return { out: 'shepherdr secret env NAME [NAME …] [--file .env]\n', code: 1 }
  const file = resolve(name)
  const git = (...a: string[]) => spawnSync('git', ['-C', dirname(file), ...a], { stdio: 'ignore' })
  const repo = git('rev-parse', '--is-inside-work-tree')
  if (repo.error) return { out: `git isn't here to check that it ignores ${name}: nothing written.\n`, code: 1 }
  if (repo.status === 0 && git('check-ignore', '-q', file).status !== 0)
    return { out: `${name} is not ignored by git: add it to .gitignore first, so the secret never lands in git.\n`, code: 1 }
  // a link (or a second name of another file) would carry it into that file
  if (existsSync(file) && (lstatSync(file).isSymbolicLink() || lstatSync(file).nlink > 1)) return { out: `${name} is a link: name a plain file, so the secret lands nowhere else.\n`, code: 1 }
  const names = Object.keys(secrets)
  const old = existsSync(file) ? readFileSync(file, 'utf8').split('\n').filter((l, n, a) => !(n === a.length - 1 && !l) && !names.some((k) => new RegExp(`^\\s*(export\\s+)?${k}\\s*=`).test(l))) : []
  // readable by you only before a value goes in: a new file from the start, an old one first
  const text = [...old, ...names.map((k) => `${k}=${envValue(secrets[k])}`)].join('\n') + '\n'
  if (existsSync(file)) {
    chmodSync(file, 0o600)
    writeFileSync(file, text)
  } else writeFileSync(file, text, { mode: 0o600, flag: 'wx' })
  return { out: `Wrote ${names.join(', ')} to ${name} (readable by you only; git ignores it). Never print or commit it.\n`, code: 0 }
}

/** a value as a .env file holds it: as it is, in single quotes (nothing in it is read), or in double quotes */
function envValue(v: string) {
  if (/^[\w@%+:,./-]*$/.test(v)) return v
  if (!/['\n\r]/.test(v)) return `'${v}'`
  return `"${v.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\$/g, '\\$').replace(/\n/g, '\\n').replace(/\r/g, '\\r')}"`
}
