// Runs INSIDE the network and PID namespace. Its only connection out is the trusted Unix gateway.
import { connect, createServer } from 'node:net'
const relay = createServer((socket) => {
  const upstream = connect('/run/shepherdr/gateway.sock')
  socket.pipe(upstream).pipe(socket)
  socket.on('error', () => upstream.destroy())
  upstream.on('error', () => socket.destroy())
})
await new Promise<void>((resolve, reject) => { relay.once('error', reject); relay.listen(18765, '127.0.0.1', resolve) })
const [binary, ...args] = process.argv.slice(2)
const env = { ...process.env, HTTP_PROXY: 'http://127.0.0.1:18765', HTTPS_PROXY: 'http://127.0.0.1:18765', ALL_PROXY: 'http://127.0.0.1:18765', NO_PROXY: '127.0.0.1', http_proxy: 'http://127.0.0.1:18765', https_proxy: 'http://127.0.0.1:18765', SHEPHERDR_URL: 'http://127.0.0.1:18765' }
const child = Bun.spawn([binary, ...args], { env, stdin: 'inherit', stdout: 'inherit', stderr: 'inherit' })
process.exit(await child.exited)
