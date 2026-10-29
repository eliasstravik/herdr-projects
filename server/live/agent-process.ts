// The CLI as herdr/ps sees it: a native binary, or npm's node/bun entry point.
export function isAgentProcess(kind: string, argv: string[] = []) {
  const name = (s: string) => s.split('/').pop() ?? ''
  if (name(argv[0] ?? '') === kind) return true
  if (!/^(node|bun)(?:\.exe)?$/.test(name(argv[0] ?? ''))) return false
  const entry = argv[1] ?? ''
  if (name(entry) === kind || name(entry) === `${kind}.js`) return true
  const packages: Record<string, string> = { pi: '/pi-coding-agent/', claude: '/claude-code/', codex: '/@openai/codex/' }
  return !!packages[kind] && entry.includes(packages[kind]) && /\.(?:m?js|cjs)$/.test(entry)
}
