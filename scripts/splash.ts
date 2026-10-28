// The launch screens an app on an iPhone's home screen shows while it starts (public/splash/), light and dark: the
// page's own background with the icon in the middle, so a dark phone doesn't flash white. iOS takes one only at the
// screen's exact size, so there is one per iPhone size; index.html's links pick them. Run by hand when the icon or the
// background changes: bun scripts/splash.ts
import { mkdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { webkit } from 'playwright-core'

const root = join(import.meta.dir, '..')
/** each iPhone's screen in points, and its pixels per point */
export const SCREENS = [
  [375, 667, 2], // SE (2nd, 3rd), 8
  [375, 812, 3], // mini, X, XS, 11 Pro
  [390, 844, 3], // 12, 13, 14, 16e
  [393, 852, 3], // 14 Pro, 15, 15 Pro, 16
  [402, 874, 3], // 16 Pro
  [414, 896, 2], // XR, 11
  [414, 896, 3], // XS Max, 11 Pro Max
  [428, 926, 3], // 12 Pro Max, 13 Pro Max, 14 Plus
  [430, 932, 3], // 14 Pro Max, 15 Plus, 15 Pro Max, 16 Plus
  [440, 956, 3], // 16 Pro Max
] as const
/** the page's background (--bg in styles.css) */
export const BG = { light: '#ffffff', dark: '#0b0b0d' }
export const splashName = (w: number, h: number, r: number, theme: keyof typeof BG) => `splash/${w}x${h}@${r}-${theme}.png`

if (import.meta.main) {
  const icon = readFileSync(join(root, 'public/icon.svg'), 'utf8')
  mkdirSync(join(root, 'public/splash'), { recursive: true })
  const browser = await webkit.launch()
  for (const [w, h, r] of SCREENS)
    for (const theme of ['light', 'dark'] as const) {
      const page = await browser.newPage({ viewport: { width: w, height: h }, deviceScaleFactor: r })
      await page.setContent(`<body style="margin:0;height:100vh;display:grid;place-items:center;background:${BG[theme]}"><div style="width:96px;height:96px">${icon.replace('<svg ', '<svg width="96" height="96" ')}</div></body>`)
      await page.screenshot({ path: join(root, 'public', splashName(w, h, r, theme)) })
      await page.close()
    }
  await browser.close()
}
