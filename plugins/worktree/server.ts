import { definePlugin } from '../../server/plugin-api.ts'

export default definePlugin({ name: 'worktree', about: "A task's worktree, read only: its files, its diff against main, its pull request" })
