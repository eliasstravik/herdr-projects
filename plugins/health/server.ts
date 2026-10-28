// Machine health: each machine samples its load, memory, disks, GPU and herdr every 30 seconds (server/live/health.ts).
// Past a limit in its Settings its coordinator hears once, with the numbers and what uses it most, and a card shows in
// its chat; again once it is back under. Off, nothing is told and only the number of working tasks holds new ones.
import { definePlugin } from '../../server/plugin-api.ts'

export default definePlugin({ name: 'health', about: "Machine health: each machine's load, memory, disk and herdr on its page; its coordinator told when one crosses its limit; new tasks held while the machine is too busy" })
