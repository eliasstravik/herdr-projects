// Voice calls go through a voice agent (ElevenLabs Conversational AI). It sends what you ask as ordinary messages to
// the agent whose chat you called from (or another, when you say so), reads the replies out, and can read every
// agent's state and recent chat. The call itself is not saved: no recording, no transcript kept (retention 0).
import { readJson, writeJson } from '../store.ts'

const API = 'https://api.elevenlabs.io/v1'

export function voiceKey(): string | null {
  return process.env.ELEVENLABS_API_KEY ?? readJson<{ elevenlabs?: string }>('secrets.json', {}).elevenlabs ?? null
}

export interface VoiceTools {
  /** who the call is with */
  name: string
  /** send to this chat (or another agent, by name); says whether it went in or waits */
  send(text: string, to?: string): Promise<string>
  /** recent chat of this agent (or another), plain */
  read(to?: string): Promise<string>
  /** every place, task and agent with its state */
  overview(): Promise<string>
  /** called when the agent you talk to writes a reply */
  onReply(fn: (text: string) => void): () => void
}

const PROMPT = `You are Shepherdr's voice agent. The human runs many coding agents from their phone and is talking to you hands-free. You pass what they ask to the agent they called ({{agent}}) as an ordinary message with send_message, word for word unless they ask you to rephrase; use the "to" field only when they name another agent. When a reply comes in, read it out briefly and naturally. To answer "where are you with that?" or "what needs me?", use read_chat or overview first. Keep everything short. Never make up what an agent said.`

async function api(path: string, key: string, init: RequestInit = {}) {
  const r = await fetch(`${API}${path}`, { ...init, headers: { 'xi-api-key': key, 'content-type': 'application/json', ...(init.headers ?? {}) } })
  if (!r.ok) throw new Error(`ElevenLabs ${path}: ${r.status} ${await r.text()}`)
  return r.json()
}

/** our voice agent at ElevenLabs, made once */
async function agentId(key: string): Promise<string> {
  const saved = readJson<{ agent?: string }>('voice.json', {})
  if (saved.agent) return saved.agent
  const tool = (name: string, description: string, props: Record<string, { type: string; description: string }>, required: string[]) => ({
    type: 'client',
    name,
    description,
    expects_response: true,
    response_timeout_secs: 20,
    parameters: { type: 'object', properties: props, required },
  })
  const body = {
    name: 'Shepherdr voice',
    conversation_config: {
      agent: {
        first_message: 'Hi, I’m listening.',
        language: 'en',
        prompt: {
          prompt: PROMPT,
          tools: [
            tool('send_message', 'Send the human’s words to an agent as a chat message.', { text: { type: 'string', description: 'What to send' }, to: { type: 'string', description: 'Another agent by name, only when the human names one' } }, ['text']),
            tool('read_chat', 'Read the recent chat and state of the agent on the call, or another one.', { to: { type: 'string', description: 'Another agent by name' } }, []),
            tool('overview', 'Every place, task and agent with its state: what needs the human, what is working.', {}, []),
          ],
        },
      },
      asr: { user_input_audio_format: 'pcm_16000' },
      tts: { agent_output_audio_format: 'pcm_16000' },
    },
    platform_settings: {
      overrides: { conversation_config_override: { agent: { prompt: { prompt: true }, first_message: true } } },
      privacy: { record_voice: false, retention_days: 0 },
    },
  }
  const r = (await api('/convai/agents/create', key, { method: 'POST', body: JSON.stringify(body) })) as { agent_id: string }
  writeJson('voice.json', { agent: r.agent_id })
  return r.agent_id
}

/** one call: the browser's microphone in, the voice agent's speech out */
export function startVoice(tools: VoiceTools, out: { send(d: ArrayBuffer | string): void; close(): void }) {
  const key = voiceKey()
  let ws: WebSocket | null = null
  let ended = false
  let offReply: (() => void) | null = null
  const fail = (msg: string) => {
    out.send(JSON.stringify({ type: 'error', message: msg }))
  }
  ;(async () => {
    if (!key) return fail('Voice needs an ElevenLabs key: run `shepherdr secret elevenlabs` on the main machine.')
    const id = await agentId(key)
    const { signed_url } = (await api(`/convai/conversation/get-signed-url?agent_id=${id}`, key)) as { signed_url: string }
    if (ended) return
    ws = new WebSocket(signed_url)
    ws.onopen = () => {
      if (ended) { ws?.close(); return }
      ws!.send(
        JSON.stringify({
          type: 'conversation_initiation_client_data',
          dynamic_variables: { agent: tools.name },
          conversation_config_override: { agent: { prompt: { prompt: PROMPT.replace('{{agent}}', tools.name) }, first_message: `You're through to ${tools.name}. Go ahead.` } },
        }),
      )
      try {
        offReply = tools.onReply((text) => !ended && ws?.readyState === WebSocket.OPEN && ws.send(JSON.stringify({ type: 'user_message', text: `(${tools.name} replied in the chat: "${text}") Read this reply out, briefly.` })))
      } catch (e) { fail((e as Error).message); ws?.close() }
    }
    ws.onmessage = async (e) => {
      if (ended) return
      const m = JSON.parse(String(e.data))
      if (m.type === 'ping') ws!.send(JSON.stringify({ type: 'pong', event_id: m.ping_event?.event_id }))
      // the screen shows what the call is doing: through, then thinking once you've said something
      else if (m.type === 'conversation_initiation_metadata') out.send(JSON.stringify({ type: 'state', state: 'listening' }))
      else if (m.type === 'user_transcript') out.send(JSON.stringify({ type: 'state', state: 'thinking' }))
      else if (m.type === 'audio' && m.audio_event?.audio_base_64) out.send(Buffer.from(m.audio_event.audio_base_64, 'base64').buffer as ArrayBuffer)
      else if (m.type === 'interruption') out.send(JSON.stringify({ type: 'interrupt' }))
      else if (m.type === 'client_tool_call') {
        const c = m.client_tool_call
        out.send(JSON.stringify({ type: 'state', state: 'thinking' }))
        let result = ''
        let is_error = false
        try {
          if (c.tool_name === 'send_message') result = await tools.send(String(c.parameters?.text ?? ''), c.parameters?.to)
          else if (c.tool_name === 'read_chat') result = await tools.read(c.parameters?.to)
          else if (c.tool_name === 'overview') result = await tools.overview()
          else ((is_error = true), (result = 'no such tool'))
        } catch (err) {
          is_error = true
          result = (err as Error).message
        }
        if (ended || ws?.readyState !== WebSocket.OPEN) return
        ws.send(JSON.stringify({ type: 'client_tool_result', tool_call_id: c.tool_call_id, result, is_error }))
      }
    }
    ws.onclose = () => {
      offReply?.()
      offReply = null
      if (!ended) { ended = true; out.close() }
    }
  })().catch((e) => fail((e as Error).message))
  return {
    audio(d: ArrayBuffer) {
      if (ws?.readyState === WebSocket.OPEN) ws.send(JSON.stringify({ user_audio_chunk: Buffer.from(d).toString('base64') }))
    },
    end() {
      ended = true
      offReply?.()
      offReply = null
      ws?.close()
    },
  }
}
