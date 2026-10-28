import { definePlugin } from '../../server/plugin-api.ts'

export default definePlugin({ name: 'tasks', about: "Tasks: one agent per task in its own worktree, started on its brief; status from that agent; shepherdr task, done, plan and merge" })
