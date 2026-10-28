// The screen built once for every browser test: the files that run in parallel share one build of the code as it is
// now, made by whichever asks first while the others wait for it. A change to the screen's code makes a new one.
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'
import { writeRegistries } from '../scripts/build-plugins.ts'
import { once } from './once.ts'

const root = join(import.meta.dir, '..')
const cache = join(root, 'node_modules', '.cache', 'shepherdr-test')
/** everything the screen is built from (scripts/test.ts runs the browser tests when one of them changes) */
export const INPUTS = ['index.html', 'vite.config.ts', 'tsconfig.json', 'package.json', 'bun.lock', 'plugins.json', 'src', 'plugins', 'shared', 'public', 'generated']

function files(path: string): string[] {
  if (!existsSync(path)) return []
  if (!statSync(path).isDirectory()) return [path]
  return readdirSync(path).sort().flatMap((n) => files(join(path, n)))
}

function key() {
  // (and how it is built: with source maps, which screen-map.ts reads)
  const h = createHash('sha256').update('sourcemap\0')
  for (const f of INPUTS.flatMap((p) => files(join(root, p)))) h.update(relative(root, f)).update('\0').update(readFileSync(f)).update('\0')
  return h.digest('hex').slice(0, 16)
}

/** the folder holding the built screen (index.html and its assets), built now if no one has yet */
export async function builtScreen(): Promise<string> {
  writeRegistries()
  const k = key()
  const dist = join(cache, `dist-${k}`)
  mkdirSync(cache, { recursive: true })
  await once(`${dist}.lock`, () => existsSync(join(dist, 'index.html')), async () => {
    const tmp = `${dist}.${process.pid}`
    const { build } = await import('vite')
    await build({ root, logLevel: 'silent', build: { outDir: tmp, emptyOutDir: true, sourcemap: 'hidden' } })
    renameSync(tmp, dist)
    // builds no one has made for a day go
    for (const n of readdirSync(cache))
      if (n.startsWith('dist-') && !n.startsWith(`dist-${k}`) && Date.now() - statSync(join(cache, n)).mtimeMs > 86_400_000) rmSync(join(cache, n), { recursive: true, force: true })
  })
  return dist
}
