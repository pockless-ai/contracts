# SessionSpend7702

ERC-7702 implementation for per-strategy session keys on EVM chains. The wallet EOA
delegates to this contract; owner-only admin calls run as `address(this)`.

## Model

Each `(strategyId, sessionKey)` session tracks:

| Field | Meaning |
| --- | --- |
| `limitUsdc` | User allocation ceiling (6-decimal USDC units) |
| `capacityUsdc` | Deployable ceiling after realized losses; profits replenish up to `limitUsdc` |
| `deployedUsdc` | Cost basis of open inventory |
| `nonce` | EIP-712 replay guard for swaps |
| `expiresAt` / `revoked` | Session lifetime |

Per-strategy asset records store `quantity` and `costUsdc` for inventory tokens.

## Swaps

Relayers call `executeSwap(intent, routerCalldata, sessionSignature)`:

1. Session key signs an EIP-712 `SwapIntent` (includes `routerCalldataHash`).
2. Calldata must target the pinned 0x AllowanceHolder (`0x000…22734`) with `exec`.
3. Spend is measured from balance deltas — not caller-supplied amounts.
4. **Buy (USDC → asset):** `deployedUsdc` increases; must fit `capacityUsdc - deployedUsdc`.
5. **Sell (asset → USDC):** pro-rata cost basis released; profit replenishes `capacityUsdc` up to `limitUsdc`; loss reduces `capacityUsdc`.

No ERC-1271, no arbitrary `execute`, no persistent allowances from the contract.

## Admin (owner = delegated EOA)

- `grant(strategyId, key, limitUsdc, expiresAt)`
- `revoke(strategyId, key)` — owner or session key; bumps `nonce`
- `rotateSession(strategyId, oldKey, newKey)`
- `setLimit(strategyId, key, newLimitUsdc, expiresAt)`

`strategyCount` / `strategyAt` / `sessionCount` / `sessionAt` support recovery indexing.

## Deploy

One deployment per chain. Constructor takes the chain’s USDC token (decimals read on-chain).
Record the address in [`../docs/deployments.json`](../docs/deployments.json).

See [`../docs/deployment.md`](../docs/deployment.md).

## Develop

```bash
cd evm
forge install foundry-rs/forge-std   # if lib/forge-std is missing
forge test
forge build
```

Tests etch a mock AllowanceHolder at the pinned mainnet address to simulate measured swaps.
