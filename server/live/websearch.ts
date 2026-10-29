import { Client, StreamableHTTPClientTransport } from '@modelcontextprotocol/client'
import { version } from '../../package.json'

const fallback = 'Parallel search is unavailable or its free limit was reached. Use another available web search tool.'

/** Anonymous search only. The plugin switch is checked before any network access. */
export async function searchWeb(query: string, enabled: boolean): Promise<string> {
  if (!enabled) throw new Error('Web search is off in Settings → Plugins → websearch.')
  query = query.trim()
  if (!query) throw new Error('Use shepherdr search "<what to find>".')
  if (query.length > 2000) throw new Error('Use at most 2,000 characters for a search.')
  const signal = AbortSignal.timeout(20_000)
  const client = new Client({ name: 'shepherdr', version })
  const transport = new StreamableHTTPClientTransport(new URL('https://mcp.parallel.ai/mcp?tools=web_search'), {
    requestInit: { redirect: 'error', credentials: 'omit' },
    fetch: (url, init) => fetch(url, { ...init, signal: AbortSignal.any([signal, ...(init?.signal ? [init.signal] : [])]) }),
  })
  try {
    await client.connect(transport, { signal, timeout: 20_000 })
    const result = await client.callTool({ name: 'web_search', arguments: {
      objective: query, search_queries: [query],
    } }, { signal, timeout: 20_000 })
    if (result.isError) throw new Error(fallback)
    const text = result.content.filter((b) => b.type === 'text').map((b) => b.text).join('\n').trim()
    return text.length > 25000 ? `${text.slice(0, 25000)}\n[Results shortened.]` : text || 'No search results.'
  } catch {
    throw new Error(fallback)
  } finally {
    await client.close().catch(() => {})
  }
}
