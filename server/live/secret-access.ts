// Secret capabilities live only in this worker's memory and the owner's private process route.
// No HTTP operation can mint one. A different worker (machine) has a separate authority.
import { randomBytes, createHash } from 'node:crypto'

export class SecretAccess {
  private owners = new Map<string, { binding: string; token: string; chain: string[] }>()
  private proofs = new Map<string, string>()
  private hash(token: string) { return createHash('sha256').update(token).digest('hex') }

  issue(chat: string, binding: string, chain: string[]) {
    const old = this.owners.get(chat)
    if (old?.binding === binding && JSON.stringify(old.chain) === JSON.stringify(chain)) return old.token
    this.revoke(chat)
    const token = randomBytes(32).toString('hex')
    this.owners.set(chat, { binding, token, chain: [...chain] })
    this.proofs.set(this.hash(token), chat)
    return token
  }

  revoke(chat: string) {
    const old = this.owners.get(chat)
    if (old) this.proofs.delete(this.hash(old.token))
    this.owners.delete(chat)
  }

  authenticate(proof?: string) {
    if (!proof || !/^[a-f0-9]{64}$/.test(proof)) return undefined
    const chat = this.proofs.get(this.hash(proof))
    const owner = chat && this.owners.get(chat)
    return owner ? { chat: chat as string, chain: [...owner.chain] } : undefined
  }
}
