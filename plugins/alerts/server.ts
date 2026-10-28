import { definePlugin } from '../../server/plugin-api.ts'

export default definePlugin({ name: 'alerts', about: "Push alerts to installed web apps: one per task, words to decide from, a tap onto the card, the app icon's count; All machines' coordinator picks what buzzes; none while the app is on screen" })
// Alerts: push notifications to the installed web app. All machines' coordinator (on Autopilot or not) picks what
// buzzes and says why.
// They only buzz when the app is closed or in the background: with the app on screen, alerts show nothing.
// What an alert says and where it opens: shared/alerts.ts; showing them: public/sw.js.
