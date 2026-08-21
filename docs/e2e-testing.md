# End-to-end testing

How to verify the full stack — contracts, SDK, recovery, and your app — before mainnet.

## Layer 1 — Contract unit tests (no chain)

### EVM

```bash
cd evm
forge install foundry-rs/forge-std   # first time
forge test --root . -vvv
```

Covers: grant (create-only), setLimit, rotate, revoke, swap accounting, replay, router policy.

### Solana

```bash
cd solana
cargo test -p strategy-spend
```

Covers: wallet/strategy init, first-vault creation, buy/sell CPI through mock Jupiter,
measured-delta limit/PnL accounting, set_limit expiry/capacity behavior, revoke, rotate,
withdraw recovery, and close.

---

## Layer 2 — Local / testnet on-chain

Use **Base Sepolia + Solana devnet** through the fixed testnet profile. Run
`yarn deploy --environment testnet --dry-run`, fund the reported public addresses manually,
then `yarn deploy --environment testnet`. Follow
[`first-deployment.md`](./first-deployment.md).

### Automated testnet smoke

After a successful testnet deploy, run the lifecycle smoke test from
`apps/contracts`:

```bash
yarn smoke:testnet
```

The command is opt-in (`LIVE_SMOKE=1` is set by the script). It reads RPC URLs and
signer paths from `deploy/.env`, loads deployed addresses from
`deploy/.deploy/testnet.json`, and exercises both chains without swaps:

- **Base Sepolia** — verify implementation bytecode and USDC config, EIP-7702 delegate the
  Foundry deployer, grant a fresh session, read `sessionOf`, revoke, then clear delegation in
  `finally`.
- **Solana devnet** — verify the program and upgrade authority, `InitWallet` if needed,
  `InitStrategy`, read the strategy account, revoke, close the flat strategy.

For the EVM leg, either enter the Foundry keystore password at the prompt or set
`EVM_FOUNDRY_PASSWORD` in `deploy/.env` when Yarn/Cursor runs without a TTY. Delegation uses
Foundry's self-broadcast flow (`cast send --auth <implementation>`) with a fixed 500k gas limit;
the first grant combines delegation and `grant` in one type-4 transaction when needed. The
Solana leg reuses the funded deployer keypair from `SOLANA_FEE_PAYER_KEYPAIR`. This validates
lifecycle behavior only; it does not execute 0x or Jupiter swaps.

### EVM smoke test

1. Deploy `SessionSpend7702` with testnet USDC.
2. Delegate a test EOA to the implementation (7702 wallet flow).
3. Owner self-call `grant(strategyId, sessionKey, limit, expiresAt)` via cast or SDK.
4. Session signs a `SwapIntent`; relayer submits `executeSwap` on testnet with a small 0x route.
5. Owner self-call `revoke`.
6. Confirm session cannot swap; owner can transfer tokens out.

```bash
# Example: read session state
cast call $OWNER_EOA \
  "sessionOf(bytes32,address)(uint128,uint128,uint128,uint64,uint64,bool,bool)" \
  $STRATEGY_ID $SESSION_KEY \
  --rpc-url $RPC_URL
```

### Solana smoke test

1. Deploy program to devnet.
2. Owner: `InitWallet`, create the vault-authority USDC ATA, USDC `approve` to the program
   authority PDA, and `InitStrategy`.
3. Fund the Solana relayer, then submit a session-and-relayer-signed `ExecuteSwap` using only
   Jupiter's returned `swapInstruction` (small route on devnet).
4. Owner: `Revoke`, `WithdrawAsset`, `CloseStrategy` when flat.

Use the SDK encoders from `sdk/` to build calldata/instruction data.

### Recovery drill

Without your backend:

```bash
cd recovery/cli
yarn install
yarn start list-strategies --chain evm --rpc $RPC --owner $OWNER
yarn start revoke --chain evm --rpc $RPC --owner $OWNER --private-key $KEY ...
```

Web UI: `cd recovery/web && yarn install && yarn dev` — connect owner wallet and repeat.

---

## Layer 3 — Application integration (monorepo)

If you integrate via the Pockless app:

1. Apply DB migration: `yarn db:migrate`
2. Set `apps/api/.env` from `deployments.json` (see [`docs/trade/go-live.md`](../../../docs/trade/go-live.md)).
3. Start local stack (`yarn dev` — Redis, Postgres, API, worker).
4. **Paper mode** — strategy authorization + trades without chain (fast regression).
5. **Live testnet** — user authorizes → grants land on-chain → small live swap → revoke via recovery.

Trade flow tests (require migrated DB + stack):

```bash
yarn workspace @workspace/trade exec tsx --test tests/strategy-authorization.test.ts
yarn workspace @workspace/trade exec tsx --test tests/execute-trade.test.ts
```

---

## Layer 4 — Pre-mainnet gate

| Check                                     | Pass criteria                                                           |
| ----------------------------------------- | ----------------------------------------------------------------------- |
| Contract tests green on release tag       | `forge test`, `cargo test -p strategy-spend`                            |
| Testnet deploy matches `deployments.json` | RPC read-back of addresses                                              |
| Recovery works without API                | Revoke + withdraw on testnet                                            |
| Relayer swap on testnet                   | Measured spend within limit                                             |
| Audit                                     | Report published **or** written risk acceptance                         |
| Verified Solana mainnet program           | `solana-verify` records the release commit and hash                     |
| Immutable Solana mainnet program          | Separate `yarn immutable --environment mainnet` reads back no authority |
| Release artifacts                         | `SHA256SUMS` matches local rebuild                                      |

---

## Suggested testnet timeline

1. **Week 1** — devnet/local: unit tests + deploy + grant/revoke only.
2. **Week 2** — testnet: one full buy + sell per chain; recovery CLI.
3. **Week 3** — app E2E in paper + live testnet; fix integration bugs.
4. **Week 4+** — audit in flight; mainnet only after fixes + retest.

Document every testnet address in `deployments.json` under a `*-testnet` key so integrators
do not confuse them with mainnet.
