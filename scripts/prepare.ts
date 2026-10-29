// Prepare a linked checkout before replacing any running server. A missing screen always gets a frozen install:
// node_modules may be a half-finished install, and is never evidence that the first build can run.
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { buildAndCheck, checkServer, run } from './update.ts'

/** Check the required packages, including their entry files, rather than the node_modules directory itself. */
export function dependenciesPresent(root: string): boolean {
  try {
    const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'))
    return Object.keys({ ...pkg.dependencies, ...pkg.devDependencies }).every((name) => {
      const dir = join(root, 'node_modules', name)
      const installed = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'))
      const paths = (value: unknown): string[] => typeof value === 'string' ? [value] :
        value && typeof value === 'object' ? Object.values(value).flatMap(paths) : []
      const entries = [installed.main, installed.module, installed.types, installed.typings,
        ...paths(installed.exports), ...paths(installed.bin)].filter((s): s is string => typeof s === 'string' && !s.includes('*'))
      return entries.every((entry) => existsSync(join(dir, entry)))
    })
  } catch {
    return false
  }
}

export async function prepare(root: string, bun = process.execPath): Promise<string | null> {
  // A manifest cannot prove that nested imports or native dependencies survived an interrupted install.
  // Reuse the release's server check before taking down an older server; a healthy checkout needs no install.
  if (existsSync(join(root, 'dist', 'index.html')) && dependenciesPresent(root) && !(await checkServer(root, bun))) return null
  const installed = await run([bun, 'install', '--frozen-lockfile'], root)
  if (!installed.ok) return `bun install failed:\n${installed.out}`
  if (!dependenciesPresent(root)) return 'bun install finished without the required dependencies; startup stopped.'
  const failed = await buildAndCheck(root, bun)
  return failed?.error ?? null
}
