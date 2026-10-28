// The demo's made-up worktrees, for a task's side panel: a few files, what each task changed, and GTM-15's pull request.
import type { Diff, DiffFile, FileText, Patch, PullRequest, WorktreeAsk } from '../../shared/types.ts'

const FILES: Record<string, string> = {
  'README.md': '# gtm\n\nGo-to-market workflows: webhooks in, enrichment, outbound.\n\n```sh\nbun install\nbun test\n```\n',
  'package.json': '{\n  "name": "gtm",\n  "private": true,\n  "type": "module",\n  "scripts": {\n    "test": "bun test",\n    "dev": "bun --watch src/server.ts"\n  }\n}\n',
  'src/server.ts':
    "import { handleWebhook } from './webhooks/handler.ts'\n\nBun.serve({\n  port: 3000,\n  fetch(req) {\n    const url = new URL(req.url)\n    if (url.pathname === '/webhooks/crm') return handleWebhook(req)\n    return new Response('not found', { status: 404 })\n  },\n})\n",
  'src/webhooks/handler.ts':
    "import { verifySignature } from './signature.ts'\n\n// A CRM webhook: checked against its signature, then queued for enrichment.\nexport async function handleWebhook(req: Request) {\n  const raw = await req.text()\n  const sig = req.headers.get('x-signature') ?? ''\n  if (!verifySignature(raw, sig, process.env.WEBHOOK_SECRET!)) return new Response('bad signature', { status: 401 })\n  const event = JSON.parse(raw) as { type: string; id: string }\n  await queue.push(event)\n  return new Response('ok')\n}\n\nconst queue: { push(e: unknown): Promise<void> } = { push: async () => {} }\n",
  'src/webhooks/signature.ts':
    "import { createHmac, timingSafeEqual } from 'node:crypto'\n\n/** the body exactly as it came, signed with the shared secret; compared in constant time */\nexport function verifySignature(raw: string, sig: string, secret: string) {\n  const want = Buffer.from(createHmac('sha256', secret).update(raw).digest('hex'))\n  const got = Buffer.from(sig)\n  return got.length === want.length && timingSafeEqual(got, want)\n}\n",
  'src/enrich/company.ts':
    "export interface Company {\n  domain: string\n  name?: string\n  employees?: number\n}\n\nexport async function enrich(domain: string): Promise<Company> {\n  return { domain }\n}\n",
  'test/signature.test.ts':
    "import { expect, test } from 'bun:test'\nimport { createHmac } from 'node:crypto'\nimport { verifySignature } from '../src/webhooks/signature.ts'\n\nconst sign = (b: string) => createHmac('sha256', 's3cret').update(b).digest('hex')\n\ntest('a body signed with the secret passes', () => {\n  expect(verifySignature('{\"a\":1}', sign('{\"a\":1}'), 's3cret')).toBe(true)\n})\n\ntest('a re-encoded body fails', () => {\n  expect(verifySignature('{ \"a\": 1 }', sign('{\"a\":1}'), 's3cret')).toBe(false)\n})\n",
  'docs/logo.png': '\0PNG',
}

/** each task's changes: its patches, by file */
const CHANGES: Record<string, { file: Omit<DiffFile, 'add' | 'del'>; patch: string }[]> = {
  'GTM-15': [
    {
      file: { path: 'src/webhooks/handler.ts', status: 'modified' },
      patch:
        "diff --git a/src/webhooks/handler.ts b/src/webhooks/handler.ts\nindex 3f1c2a9..8be41d0 100644\n--- a/src/webhooks/handler.ts\n+++ b/src/webhooks/handler.ts\n@@ -1,10 +1,11 @@\n import { verifySignature } from './signature.ts'\n \n // A CRM webhook: checked against its signature, then queued for enrichment.\n export async function handleWebhook(req: Request) {\n-  const event = await req.json()\n-  const sig = req.headers.get('x-signature')\n-  if (!verifySignature(JSON.stringify(event), sig, process.env.WEBHOOK_SECRET!)) return new Response('bad signature', { status: 401 })\n+  const raw = await req.text()\n+  const sig = req.headers.get('x-signature') ?? ''\n+  if (!verifySignature(raw, sig, process.env.WEBHOOK_SECRET!)) return new Response('bad signature', { status: 401 })\n+  const event = JSON.parse(raw) as { type: string; id: string }\n   await queue.push(event)\n   return new Response('ok')\n }\n",
    },
    {
      file: { path: 'src/webhooks/signature.ts', status: 'modified' },
      patch:
        "diff --git a/src/webhooks/signature.ts b/src/webhooks/signature.ts\nindex 77ad0e1..c09b5f2 100644\n--- a/src/webhooks/signature.ts\n+++ b/src/webhooks/signature.ts\n@@ -1,6 +1,8 @@\n-import { createHmac } from 'node:crypto'\n+import { createHmac, timingSafeEqual } from 'node:crypto'\n \n-export function verifySignature(body: string, sig: string | null, secret: string) {\n-  return createHmac('sha256', secret).update(body).digest('hex') !== sig ? false : true\n+/** the body exactly as it came, signed with the shared secret; compared in constant time */\n+export function verifySignature(raw: string, sig: string, secret: string) {\n+  const want = Buffer.from(createHmac('sha256', secret).update(raw).digest('hex'))\n+  const got = Buffer.from(sig)\n+  return got.length === want.length && timingSafeEqual(got, want)\n }\n",
    },
    {
      file: { path: 'test/signature.test.ts', status: 'added' },
      patch:
        "diff --git a/test/signature.test.ts b/test/signature.test.ts\nnew file mode 100644\nindex 0000000..5a1e9c3\n--- /dev/null\n+++ b/test/signature.test.ts\n@@ -0,0 +1,13 @@\n+import { expect, test } from 'bun:test'\n+import { createHmac } from 'node:crypto'\n+import { verifySignature } from '../src/webhooks/signature.ts'\n+\n+const sign = (b: string) => createHmac('sha256', 's3cret').update(b).digest('hex')\n+\n+test('a body signed with the secret passes', () => {\n+  expect(verifySignature('{\"a\":1}', sign('{\"a\":1}'), 's3cret')).toBe(true)\n+})\n+\n+test('a re-encoded body fails', () => {\n+  expect(verifySignature('{ \"a\": 1 }', sign('{\"a\":1}'), 's3cret')).toBe(false)\n+})\n",
    },
    {
      file: { path: 'src/webhooks/legacy.ts', status: 'deleted' },
      patch:
        "diff --git a/src/webhooks/legacy.ts b/src/webhooks/legacy.ts\ndeleted file mode 100644\nindex 1b2c3d4..0000000\n--- a/src/webhooks/legacy.ts\n+++ /dev/null\n@@ -1,3 +0,0 @@\n-// the old v1 handler, kept for one release\n-export const LEGACY = true\n-export const handleV1 = () => new Response('gone', { status: 410 })\n",
    },
  ],
}

const PRS: Record<string, PullRequest> = {
  'GTM-15': {
    number: 128,
    title: 'Fix webhook signature check',
    state: 'open',
    url: 'https://github.com/example/gtm/pull/128',
    checks: [
      { name: 'test', state: 'pass', url: 'https://github.com/example/gtm/actions/runs/1' },
      { name: 'typecheck', state: 'pass' },
      { name: 'preview deploy', state: 'pending' },
    ],
  },
}

const counts = (patch: string) => {
  let add = 0
  let del = 0
  for (const l of patch.split('\n')) {
    if (l.startsWith('+++') || l.startsWith('---')) continue
    if (l.startsWith('+')) add++
    else if (l.startsWith('-')) del++
  }
  return { add, del }
}

/** the made-up worktree of a demo task: every task shares the gtm files; only some changed anything */
export function demoWorktree(ask: WorktreeAsk, has: boolean): unknown {
  if (!has) throw new Error(`No task ${ask.task}.`)
  const changes = CHANGES[ask.task] ?? []
  if (ask.what === 'files') return { files: Object.keys(FILES).sort() }
  if (ask.what === 'file') {
    const text = FILES[ask.path]
    if (text === undefined) throw new Error('No such file.')
    return (text.includes('\0') ? { path: ask.path, size: 2048, binary: true } : { path: ask.path, size: text.length, text }) satisfies FileText
  }
  if (ask.what === 'diff') {
    const files = changes.map((c) => ({ ...c.file, ...counts(c.patch) }))
    return { base: 'main', files, add: files.reduce((x, f) => x + f.add, 0), del: files.reduce((x, f) => x + f.del, 0) } satisfies Diff
  }
  if (ask.what === 'patch') {
    const c = changes.find((x) => x.file.path === ask.path)
    if (!c) throw new Error('That file has no changes.')
    return { path: ask.path, text: c.patch } satisfies Patch
  }
  const pr = PRS[ask.task]
  return pr ? { pr } : { pr: null, why: 'No pull request yet.' }
}

/** the demo tasks with a pull request open */
export const DEMO_PRS = Object.fromEntries(Object.entries(PRS).map(([id, p]) => [id, { number: p.number, url: p.url }]))
