import { definePlugin } from '../../server/plugin-api.ts'

export default definePlugin({ name: 'coordinators', about: "One coordinator agent per place, started and resumed through herdr; modes and the news that wakes them" })
