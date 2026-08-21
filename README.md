# Contracts

On-chain programs for **session-key trading with owner recovery**: a wallet owner delegates
limited, revocable swap authority to a session key while retaining direct on-chain control
to revoke, withdraw, and inspect state — without any backend.

Published at [`github.com/pockless-ai/contracts`](https://github.com/pockless-ai/contracts).

## What’s in this repo

| Path                                                                  | Description                                                                                                                                                    |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`evm/`](./evm)                                                       | `SessionSpend7702` — EIP-7702 + EIP-712 swap intents, pinned 0x AllowanceHolder                                                                                |
| [`solana/programs/strategy-spend/`](./solana/programs/strategy-spend) | Per-strategy vaults, Jupiter CPI, measured USDC limits                                                                                                         |
| [`sdk/`](./sdk)                                                       | `@pockless/protocol-sdk` — ABIs, Borsh encoders, PDA helpers                                                                                                   |
| [`deploy/`](./deploy)                                                 | Resumable secure deployment and Solana immutability CLI                                                                                                        |
| [`recovery/web/`](./recovery/web)                                     | Static React UI for owner recovery (GitHub Pages)                                                                                                              |
| [`recovery/cli/`](./recovery/cli)                                     | Local CLI for owner-signed revoke / withdraw / close                                                                                                           |
| [`docs/`](./docs)                                                     | Deployment, [first deploy](./docs/first-deployment.md), [audit](./docs/audit.md), [E2E testing](./docs/e2e-testing.md), [token policy](./docs/token-policy.md) |

One **implementation address per EVM chain** and one **program id on Solana**. Owners delegate
their existing wallet; do not deploy a copy per strategy or per user.

## Quick start

```bash
git clone https://github.com/pockless-ai/contracts.git
cd contracts
yarn setup
```

### EVM

`yarn setup` installs Foundry and initializes `forge-std`.

```bash
cd evm
forge test --root .
```

### Solana

`yarn setup` installs Rust, Solana platform tools, and `solana-verify`.

```bash
cd solana
cargo test --locked -p strategy-spend
solana-verify build .
```

### SDK

```bash
cd sdk
yarn install
yarn typecheck
```

## Deploy

Copy `deploy/.env.example` to your private environment configuration, restore signer files
from encrypted storage, and run:

```bash
yarn deploy --environment testnet --dry-run
yarn deploy --environment testnet
yarn deploy --environment mainnet
yarn upgrade --environment testnet --dry-run
yarn upgrade --environment testnet
```

Mainnet is interactive, refuses CI/non-TTY execution, and uses an encrypted Foundry account
plus file-path Solana keypairs. It never accepts raw keys or funds deployers. Upgrades deploy
new immutable EVM implementations and update Solana at its existing program ID. After public
Solana verification and a separate review, permanently remove its upgrade authority with:

```bash
yarn immutable --environment mainnet
```

See [`docs/deployment.md`](./docs/deployment.md) and
[`docs/first-deployment.md`](./docs/first-deployment.md). Immutability is irreversible and
is never part of deployment.

After deploy, publish a GitHub **Release** (signed tarballs + `SHA256SUMS` via CI) and enable
**GitHub Pages** for the recovery web.

## SDK

Install from npm (when published) or build from source:

```bash
cd sdk
yarn install
yarn typecheck
```

Import from `@pockless/protocol-sdk`:

- EVM: `encodeSessionGrant`, `encodeSessionRevoke`, `encodeExecuteSwap`, `strategyIdFromCuid`, …
- Solana: `strategyPda`, `encodeInitStrategy`, `encodeExecuteSwap`, `solanaStrategyIdFromCuid`, …

Load pinned addresses from [`docs/deployments.json`](./docs/deployments.json) (or your own manifest).

## Owner recovery

Recovery works with **any public RPC** and the owner wallet only — no API, relayer, or integrator
service required.

| Tool   | Docs                                     |
| ------ | ---------------------------------------- |
| Guide  | [`docs/recovery.md`](./docs/recovery.md) |
| Web UI | [`recovery/web/`](./recovery/web/)       |
| CLI    | [`recovery/cli/`](./recovery/cli/)       |

**Web** (local dev):

```bash
cd recovery/web
yarn install
yarn dev
```

**CLI** example:

```bash
cd recovery/cli
yarn install
yarn start list-strategies \
  --chain evm --rpc https://mainnet.base.org --owner 0xYourWallet
```

Run `yarn start --help` from `recovery/cli` for all commands.

## Threat model

See [`docs/threat-model.md`](./docs/threat-model.md).

## CI

| Workflow                          | Purpose                                                     |
| --------------------------------- | ----------------------------------------------------------- |
| `.github/workflows/contracts.yml` | Foundry, Solana, deploy CLI, and SDK checks (never deploys) |
| `.github/workflows/pages.yml`     | Recovery web → GitHub Pages                                 |
| `.github/workflows/release.yml`   | Release artifacts + checksums                               |

## Integrating in an application

This repository is **self-contained**. Typical integration:

1. Add `@pockless/protocol-sdk` to your app (`npm install @pockless/protocol-sdk`).
2. Read deployment addresses from `docs/deployments.json` or your config.
3. Your app prepares owner-signed grants and stores session keys off-chain; a relayer (yours)
   submits EVM `executeSwap` and Solana vault swaps.

See [`docs/integration.md`](./docs/integration.md) for end-to-end flows and identifiers.

## License

MIT — see [`LICENSE`](./LICENSE).
