# Owner recovery

Owners can revoke sessions, withdraw vault inventory, and close strategies using only a
**public RPC** and their wallet key. No backend, API key, session JWT, or integrator service
is required.

## Prerequisites

- Owner wallet (EVM private key or Solana keypair)
- Public RPC URL for the relevant chain
- Deployment addresses from [`deployments.json`](./deployments.json) (verify against the
  [release](https://github.com/pockless-ai/contracts/releases) you trust)

## Tools

| Tool | Use case |
| --- | --- |
| [Recovery web](../recovery/web/) | Browser UI — connect wallet, list, revoke, withdraw, close |
| [Recovery CLI](../recovery/cli/) | Scriptable recovery for air-gapped or automated workflows |

### Web

```bash
cd recovery/web
yarn install
yarn dev
```

Production build (GitHub Pages base path):

```bash
cd recovery/web
yarn install
GITHUB_PAGES=true yarn build
```

Published builds are served from the repository GitHub Pages site.

### CLI

From the repository root:

```bash
cd recovery/cli
yarn install

yarn start list-strategies \
  --chain evm --rpc https://mainnet.base.org --owner 0xYourWallet

yarn start list-strategies \
  --chain solana --rpc https://api.mainnet-beta.solana.com \
  --owner YourBase58Pubkey --program-id <PROGRAM_ID>

yarn start revoke \
  --chain evm --rpc ... --owner 0x... --strategy-id 0x... --session-key 0x...

yarn start withdraw \
  --chain solana --rpc ... --owner ... --strategy-id <hex32> \
  --mint <tokenMint> --amount 1000000

yarn start close \
  --chain solana --rpc ... --owner ... --strategy-id <hex32>
```

Run `yarn start --help` from `recovery/cli` for all flags.

## EVM (SessionSpend7702)

The owner EOA delegates to the chain implementation. Owner admin calls are transactions
**to the owner’s own address** with calldata from `@pockless/protocol-sdk`.

### List sessions

On the **owner wallet address** (delegated EOA), read `strategyCount`, `strategyAt`,
`sessionCount`, `sessionAt`, and `sessionOf`.

### Revoke

Owner-signed tx to self with `encodeSessionRevoke({ strategyId, sessionAddress })`.
Stops the session key from executing further swaps.

### Withdraw tokens

Tokens from swaps sit in the owner EOA balance. Transfer ERC-20 with a standard `transfer`.
Revoke active sessions first if assets were acquired through automated swaps.

There is no on-chain “close strategy” on EVM — revoke sessions and transfer balances.

## Solana (strategy-spend)

### List strategies

Fetch `StrategyAccount` PDAs for the owner via `getProgramAccounts`.

### Revoke

Owner-signed `encodeRevoke()` — accounts `[authority, strategy]` where authority is owner
or session.

### Withdraw

Owner-signed `encodeWithdrawAsset(amount)` — vault ATA → owner ATA.

### Close

Owner-signed `encodeCloseStrategy()` when `deployed_usdc == 0`. Withdraw or sell inventory
first.

## Verification checklist

1. Confirm RPC URL and chain id / cluster.
2. Compare program / implementation address to [`deployments.json`](./deployments.json)
   and release checksums.
3. Revoke active sessions before large withdrawals.
4. Cross-check balances on a second RPC provider.

See [`threat-model.md`](./threat-model.md).
