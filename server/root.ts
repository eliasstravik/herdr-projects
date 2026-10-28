// The checkout herdr links. The server itself runs from a frozen release of it (.releases/, see scripts/update.ts),
// which goes away a couple of updates later; what agents keep using (the shepherdr command, the skill) points here.
import { join } from 'node:path'

export const ROOT = process.env.SHEPHERDR_ROOT ?? join(import.meta.dir, '..')
