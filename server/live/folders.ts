// Adding a project from the sidebar: the human browses this machine's folders and picks a repo, or makes a new one.
// Only folders inside the human's home are listed or made: a path is resolved (~, .., links) and refused if it leads
// anywhere else. Folder names only; nothing inside a file is read.
import { BOX, boxHome, q, runSync } from './box.ts'
import { existsSync, lstatSync, mkdirSync, readdirSync, realpathSync, statSync } from './mfs.ts'
import { basename, dirname, join, resolve, sep } from 'node:path'
import type { FolderEntry, Folders } from '../../shared/types.ts'
import { git, sh, topLevel } from './engine/git.ts'

/** where repos usually live, looked through for likely ones */
const ROOTS = ['dev', 'code', 'Code', 'src', 'projects', 'Projects', 'repos', 'work', 'git', 'github', 'Developer', '']
const MAX_ENTRIES = 400

/** the human's home, as their shell has it */
export function home() {
  return realpathSync(BOX ? boxHome() : process.env.HOME || boxHome())
}

/** ~/dev/x for a path in home */
export function tilde(abs: string, h = home()) {
  return abs === h ? '~' : abs.startsWith(h + sep) ? `~/${abs.slice(h.length + 1)}` : abs
}

const inside = (abs: string, h: string) => abs === h || abs.startsWith(h + sep)

/**
 * A typed path as a real place in home: ~ and relative paths start at home; .. and links are followed before the
 * check, so no spelling leads out. A folder not made yet is checked by the nearest folder above it that is there.
 */
export function resolveInHome(input: string, h = home()): { abs: string; exists: boolean } {
  const raw = input.trim()
  if (raw.includes('\0')) throw new Error('That is not a folder path.')
  const spelled = resolve(h, raw.replace(/^~(?=$|\/)/, '.'))
  let at = spelled
  let rest = ''
  while (!existsSync(at)) {
    // a link that leads nowhere could still lead out once something is made there
    if (lstatOk(at)) throw new Error('Only folders in your home can be added.')
    rest = rest ? join(basename(at), rest) : basename(at)
    const up = dirname(at)
    if (up === at) break
    at = up
  }
  let real: string
  try {
    real = realpathSync(at)
  } catch {
    throw new Error('Only folders in your home can be added.')
  }
  const abs = rest ? join(real, rest) : real
  if (!inside(abs, h)) throw new Error('Only folders in your home can be added.')
  return { abs, exists: !rest }
}

const lstatOk = (p: string) => {
  try {
    lstatSync(p)
    return true
  } catch {
    return false
  }
}

const isDir = (p: string) => {
  try {
    return statSync(p).isDirectory()
  } catch {
    return false
  }
}

const hasGit = (p: string) => existsSync(join(p, '.git'))

/** when a repo was last worked in: its index or HEAD changes with every commit and checkout */
function touched(repo: string) {
  let t = 0
  for (const f of ['.git/index', '.git/HEAD', '.git']) {
    try {
      t = Math.max(t, statSync(join(repo, f)).mtimeMs)
    } catch {
      /* not there */
    }
  }
  return t
}

/** another machine's folders, read in one go: name, real path, whether it is a repo, when it was last worked in */
const REMOTE_LIST = `perl -e 'my $d = $ARGV[0]; opendir(my $h, $d) or exit 1; use Cwd "abs_path"; for my $n (sort readdir $h) { next if $n eq "." || $n eq ".."; my $p = "$d/$n"; next unless -d $p; my $r = abs_path($p) // next; my $t = 0; for my $f (".git/index", ".git/HEAD", ".git") { my @s = stat("$p/$f"); $t = $s[9] if @s && $s[9] > $t } print join("\\t", $n, $r, (-e "$p/.git" ? 1 : 0), $t * 1000), "\\n" }'`

function subfolders(abs: string, h: string, added: Set<string>): (FolderEntry & { at: number })[] {
  if (BOX) {
    const r = runSync(['/bin/sh', '-c', `${REMOTE_LIST} ${q(abs)}`])
    if (r.code) return []
    return r.out
      .toString()
      .split('\n')
      .filter(Boolean)
      .map((l) => l.split('\t'))
      .filter(([, real]) => inside(real, h))
      .map(([name, real, repo, at]) => ({ name, path: tilde(join(abs, name), h), repo: repo === '1', added: added.has(real) || added.has(join(abs, name)) || undefined, at: repo === '1' ? Number(at) : 0 }))
  }
  let names: string[]
  try {
    names = readdirSync(abs)
  } catch {
    return []
  }
  const out: (FolderEntry & { at: number })[] = []
  for (const name of names) {
    const p = join(abs, name)
    if (!isDir(p)) continue
    // a link out of home is not shown
    let real: string
    try {
      real = realpathSync(p)
    } catch {
      continue
    }
    if (!inside(real, h)) continue
    const repo = hasGit(p)
    out.push({ name, path: tilde(p, h), repo, added: added.has(real) || added.has(p) || undefined, at: repo ? touched(p) : 0 })
  }
  return out
}

/** repos first, the latest worked in on top; then the other folders by name */
const order = (a: FolderEntry & { at: number }, b: FolderEntry & { at: number }) =>
  Number(b.repo) - Number(a.repo) || b.at - a.at || a.name.localeCompare(b.name, undefined, { sensitivity: 'base' })

/** a folder and the folders in it; at home also the repos most likely meant */
export async function listFolders(input: string, projects: string[] = [], h = home()): Promise<Folders> {
  const { abs, exists } = resolveInHome(input || '~', h)
  const added = new Set(projects)
  const dir = exists && isDir(abs)
  const entries = dir ? subfolders(abs, h, added).sort(order) : []
  const top = dir ? await topLevel(abs) : null
  const out: Folders = {
    path: tilde(abs, h),
    exists,
    dir,
    repo: top ? tilde(top, h) : null,
    added: added.has(abs) || (top ? added.has(top) : false) || undefined,
    entries: entries.slice(0, MAX_ENTRIES).map(({ at: _, ...e }) => e),
    more: entries.length > MAX_ENTRIES || undefined,
  }
  if (abs === h) out.likely = likelyRepos(h, added)
  return out
}

/** repos under the usual folders, the latest worked in first; ones already added are left out */
export function likelyRepos(h: string, added: Set<string>, max = 6): FolderEntry[] {
  const seen = new Set<string>()
  const all: (FolderEntry & { at: number })[] = []
  for (const r of ROOTS) {
    const root = join(h, r)
    if (!isDir(root)) continue
    for (const e of subfolders(root, h, added)) {
      if (!e.repo || e.added || seen.has(e.path)) continue
      seen.add(e.path)
      all.push(e)
    }
  }
  return all
    .sort((a, b) => b.at - a.at)
    .slice(0, max)
    .map(({ at: _, ...e }) => e)
}

/**
 * Make a folder in home a new repo: the folder (if it isn't there yet), git init on main and a first commit. The
 * commit is empty: files already in the folder stay as they are, uncommitted.
 */
export async function makeRepo(input: string, h = home()): Promise<string> {
  const { abs, exists } = resolveInHome(input, h)
  if (abs === h) throw new Error('Pick or name a folder inside your home, not home itself.')
  if (exists && !isDir(abs)) throw new Error(`${tilde(abs, h)} is a file, not a folder.`)
  if (!exists) mkdirSync(abs, { recursive: true })
  // made: look again, with every link in it followed
  if (!inside(realpathSync(abs), h)) throw new Error('Only folders in your home can be added.')
  if (await topLevel(abs)) return abs
  let r = await git(abs, 'init', '-q', '-b', 'main')
  // git before 2.28 has no -b
  if (!r.ok) r = await git(abs, 'init', '-q')
  if (!r.ok) throw new Error(`git init failed: ${r.err}`)
  await git(abs, 'symbolic-ref', 'HEAD', 'refs/heads/main')
  const c = await sh(['git', '-c', 'user.useConfigOnly=false', 'commit', '-q', '--allow-empty', '-m', 'First commit'], abs)
  if (!c.ok) throw new Error(`The first commit failed: ${c.err}`)
  return abs
}
