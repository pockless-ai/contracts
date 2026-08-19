# Token policy

Which tokens the protocol supports and how to handle non-standard ERC-20 / SPL behavior.

## Supported model

| Leg | Token | Accounting |
| --- | --- | --- |
| Quote | **USDC only** (pinned mint per chain) | `limitUsdc`, `capacityUsdc`, `deployedUsdc` |
| Inventory | Tokens acquired via **pinned router only** | Per-strategy quantity + cost basis |

Swaps must be **USDC ↔ inventory token**. No USDC↔USDC, no arbitrary token↔token.

## Standard tokens (supported)

Use **plain ERC-20 / SPL** tokens that:

- Transfer exact amounts (no fee on transfer)
- Do not rebase or change balances outside transfers
- Have stable decimals (inventory decimals arbitrary; USDC normalized to 6 decimals in protocol math)

Most assets routed through 0x / Jupiter on major pairs satisfy this.

## Unsupported (do not treat as safe)

| Type | Problem | On-chain behavior |
| --- | --- | --- |
| **Fee-on-transfer** | Balance delta &lt; stated amount | Swap may **revert** (safe) or accounting may drift if partial |
| **Rebasing** (stETH-style) | `quantity` no longer matches economic exposure | Cost basis wrong; limits meaningless |
| **Tokens with hooks** | Unexpected callbacks | Out of scope; router may fail |
| **Malicious tokens** | Fake USDC, infinite mint | Mitigated by pinning **canonical USDC** at deploy / InitWallet |

## Recommendation

### Product / integrator (off-chain)

1. **Allowlist inventory** — only execute swaps for tokens you have reviewed (e.g. top liquidity pairs on 0x/Jupiter).
2. **Block known fee-on-transfer** — maintain a denylist from public lists (e.g. USDT on some chains if applicable).
3. **Display accounting unit** — users authorize **USDC notional**, not “any token value.”

### Protocol (on-chain) — current

- EVM: accounting uses **balance deltas** around AllowanceHolder `exec`, not transfer return values.
- Solana: post-CPI **exact** spent/received checks against declared amounts.
- Non-USDC↔token legs **revert**.

This is **fail-closed for weird pairs** in most cases (swap reverts), but does not guarantee
useful behavior for exotic tokens.

### Optional future hardening (not implemented)

- On-chain **mint allowlist** per strategy (owner-configured).
- Minimum liquidity / pool checks (usually off-chain in route selection).
- Explicit `supportsToken(mint)` view fed from governance.

We have not added an on-chain allowlist to avoid scope creep; **integrator route selection**
is the first line of defense.

## User-facing guidance

Tell users:

- Authorization limits apply to **USDC deployment**, not mark-to-market of inventory.
- Only trade tokens your service routes through **0x (EVM)** or **Jupiter (Solana)**.
- Exotic or new tokens may fail to swap or recover cleanly — stick to liquid pairs until
  you explicitly support a token.

See [`threat-model.md`](./threat-model.md) for trust boundaries.
