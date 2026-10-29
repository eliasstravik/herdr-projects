import { afterEach, expect, test } from 'bun:test'
import { searchWeb } from '../server/live/websearch.ts'
import { LiveSource } from '../server/live/source.ts'

afterEach(() => { globalThis.fetch = originalFetch })
const originalFetch = globalThis.fetch

// Exercise the real MCP client against an HTTP boundary fixture; no live service or account.
function server(result: unknown, status = 200) {
  const calls: { request: Request; body: Record<string, any> }[] = []
  globalThis.fetch = Object.assign(async (input: Parameters<typeof fetch>[0], init?: Parameters<typeof fetch>[1]) => {
    const request = new Request(input, init)
    const body = await request.json().catch(() => ({})) as Record<string, any>
    calls.push({ request, body })
    if (body.method === 'initialize') return Response.json({ jsonrpc: '2.0', id: body.id, result: {
      protocolVersion: '2025-11-25', capabilities: { tools: {} }, serverInfo: { name: 'fixture', version: '1' },
    } })
    if (body.method === 'notifications/initialized') return new Response(null, { status: 202 })
    if (body.method === 'tools/call') return status === 200
      ? Response.json({ jsonrpc: '2.0', id: body.id, result })
      : new Response('Free limit reached', { status })
    return new Response(null, { status: 405 })
  }, { preconnect: originalFetch.preconnect })
  return calls
}

test('search returns source URLs and excerpts through the anonymous MCP', async () => {
  const calls = server({ content: [{ type: 'text', text: 'https://example.com/docs\nCurrent documentation.' }] })
  expect(await searchWeb('current documentation', true)).toBe('https://example.com/docs\nCurrent documentation.')
  const call = calls.find((x) => x.body.method === 'tools/call')!
  expect(call.request.url).toBe('https://mcp.parallel.ai/mcp?tools=web_search')
  expect(call.request.headers.has('authorization')).toBe(false)
  expect(call.body.params.name).toBe('web_search')
  expect(call.body.params.arguments).toEqual({ objective: 'current documentation', search_queries: ['current documentation'] })
})

test('the CLI keeps search terms that look like command options', async () => {
  const calls = server({ content: [{ type: 'text', text: 'https://example.com/docs' }] })
  const source = Object.create(LiveSource.prototype) as LiveSource
  expect((await source.cli(['search', 'bun', '--coverage', 'flags'], {})).code).toBe(0)
  expect(calls.find((x) => x.body.method === 'tools/call')!.body.params.arguments.search_queries).toEqual(['bun --coverage flags'])
})

test('disabled search and invalid queries make no network request', async () => {
  const calls = server({ content: [] })
  await expect(searchWeb('docs', false)).rejects.toThrow('Settings → Plugins')
  await expect(searchWeb('   ', true)).rejects.toThrow('shepherdr search')
  await expect(searchWeb('x'.repeat(2001), true)).rejects.toThrow('2,000')
  expect(calls).toHaveLength(0)
})

test('the free limit and tool errors tell the agent to use another search tool', async () => {
  const limited = server({}, 429)
  await expect(searchWeb('docs', true)).rejects.toThrow('another available web search tool')
  expect(limited.at(-1)!.body.method).toBe('tools/call')
  globalThis.fetch = originalFetch
  const failed = server({ isError: true, content: [{ type: 'text', text: 'Search failed' }] })
  await expect(searchWeb('docs', true)).rejects.toThrow('another available web search tool')
  expect(failed.some((x) => x.body.method === 'tools/call')).toBe(true)
})

test('empty results are clear and long output is bounded', async () => {
  server({ content: [] })
  expect(await searchWeb('docs', true)).toBe('No search results.')
  globalThis.fetch = originalFetch
  server({ content: [{ type: 'text', text: 'x'.repeat(30000) }] })
  const text = await searchWeb('docs', true)
  expect(text).toEndWith('[Results shortened.]')
  expect(text.length).toBeLessThan(25100)
})
