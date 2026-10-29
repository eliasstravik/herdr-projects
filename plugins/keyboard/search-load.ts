import type { Chat } from '../../shared/types.ts'

/** Read the existing private chat stream in small batches, without replacing the screen's subscriptions.
 * Results live only for this open search. A timeout is a coverage gap, never a claim that a chat was empty. */
export function loadSearchChats(keys: string[], receive: (chat: Chat) => void, progress: (done: number, missing: string[], finished: boolean) => void) {
  const url = new URL('api/ws', location.href)
  url.protocol = url.protocol.replace('http', 'ws')
  const socket = new WebSocket(url)
  let stopped = false
  let offset = 0
  let pending = new Set<string>()
  const missing: string[] = []
  let timer: ReturnType<typeof setTimeout>
  const finish = () => {
    clearTimeout(timer)
    if (stopped) return
    stopped = true
    progress(offset - pending.size, [...missing, ...pending, ...keys.slice(offset)], true)
    socket.close()
  }
  const batch = () => {
    if (stopped) return
    missing.push(...pending)
    pending = new Set(keys.slice(offset, offset + 6))
    offset += pending.size
    progress(offset - pending.size, [...missing], !pending.size)
    if (!pending.size) return finish()
    socket.send(JSON.stringify({ type: 'watch', chats: [...pending], term: null, visible: false }))
    timer = setTimeout(batch, 5000)
  }
  timer = setTimeout(finish, 8000)
  socket.onopen = () => { clearTimeout(timer); batch() }
  socket.onmessage = e => {
    if (stopped) return
    try {
      const msg = JSON.parse(e.data)
      if (msg.type !== 'chat' || !pending.has(msg.chat.key)) return
      receive(msg.chat)
      pending.delete(msg.chat.key)
      progress(offset - pending.size, [...missing], false)
      if (!pending.size) { clearTimeout(timer); timer = setTimeout(batch, 0) }
    } catch { /* an invalid update cannot claim coverage */ }
  }
  socket.onerror = finish
  socket.onclose = finish
  return () => { stopped = true; clearTimeout(timer); socket.close() }
}
