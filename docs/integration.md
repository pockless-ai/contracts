# Integration

How to use the **contracts** repo from an application. This repo does not include a backend;
integrators supply authorization UX, session key storage, relayers, and indexing.

## Dependencies

```bash
npm install @pockless/protocol-sdk
```

Import encoders and constants from `@pockless/protocol-sdk`. Load deployment addresses from
[`deployments.json`](./deployments.json) for the release you pin.

To hack on the SDK from a checkout of this repo:

```bash
cd sdk
yarn install
yarn typecheck
```

## Identifiers

- **EVM & Solana strategy id:** `keccak256(UTF-8 integratorStrategyId)` — 32 bytes.
  The integrator chooses an opaque string id; the protocol only sees the hash.

## EVM flow

1. **Deploy** one `SessionSpend7702` per chain ([`deployment.md`](./deployment.md)).
2. **Owner delegates** their EOA to the implementation (EIP-7702).
3. **Grant:** owner self-call `grant(strategyId, sessionKey, limitUsdc, expiresAt)`.
4. **Swap:** session signs EIP-712 `SwapIntent`; any relayer calls `executeSwap` on the
   owner EOA with 0x AllowanceHolder calldata.
5. **Revoke / rotate / setLimit:** owner self-call; see SDK encoders. Use `setLimit` for limit
   and expiry changes on existing sessions; `grant` is create-only.

Session state is readable via `sessionOf(strategyId, sessionKey)` on the owner address.

## Solana flow

1. **Deploy** `strategy-spend` and record `programId`.
2. **InitWallet** (once per owner) — pins USDC mint and Jupiter program id.
3. **InitStrategy + vault setup** — owner creates the strategy PDA, creates the
   vault-authority USDC ATA, approves the program authority, and sets the session/limit.
4. **ExecuteSwap** — a funded relayer pays the transaction fee and first-asset ATA/PDA rent;
   the session also signs. Request Jupiter `swap-instructions` with the vault-authority PDA
   as `userPublicKey`, `useSharedAccounts: false`, and the strategy destination ATA. Pass only
   the returned `swapInstruction` accounts/data, preserving account order.
5. **Revoke / withdraw / close** — owner-only recovery paths.

PDAs: see [`sdk/src/solana/strategy-spend.ts`](../sdk/src/solana/strategy-spend.ts) and the
program README under `solana/programs/strategy-spend/`.

## Recovery

Owners are never locked to your service. Ship links to the [recovery web](../recovery/web/)
and [CLI](../recovery/cli/), or document RPC + SDK steps from [`recovery.md`](./recovery.md).

## Releases

Pin integrator deployments to a **git tag** and matching `deployments.json`. Verify bytecode
and program hashes from GitHub Release `SHA256SUMS`.
