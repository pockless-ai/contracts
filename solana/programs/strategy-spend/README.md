# StrategySpend

Solana program for per-strategy session keys, USDC limits, inventory vaults, and pinned
Jupiter swaps. Part of [pockless-ai/contracts](https://github.com/pockless-ai/contracts).

SPL token accounts allow one delegate. This program is the USDC delegate on the owner’s
ATA, but Jupiter never receives that account. A buy first moves its bounded input into the
strategy’s program-controlled USDC vault. Each strategy has a PDA with its own session key,
ceiling, capacity accounting, USDC vault, and inventory vaults.

Record the deployed program id in [`../../docs/deployments.json`](../../docs/deployments.json).
See [`../../docs/deployment.md`](../../docs/deployment.md).

## Accounts

| PDA | Seeds |
| --- | --- |
| Wallet config | `["wallet", owner]` |
| Program authority (USDC delegate) | `["authority", owner]` |
| Strategy | `["strategy", owner, strategy_id]` |
| Vault authority | `["vault", strategy]` |
| Strategy asset ledger | `["asset", strategy, mint]` |

Strategy vault token accounts are ATAs owned by the vault authority PDA. Authorization
creates the USDC vault owner-funded. ExecuteSwap creates a missing inventory ATA and asset
PDA with the funded relayer as payer, while still requiring the session signature.

## State

**WalletConfig** — `version`, `owner`, `usdc_mint`, pinned `token_program`,
`associated_token_program`, `jupiter_program`, `authority_bump`.

**StrategyAccount** — `strategy_id[32]`, `owner`, `session`, `limit_usdc`,
`capacity_usdc`, `deployed_usdc`, `expires_at`, `nonce`, `revoked`, `vault_bump`.

**StrategyAsset** — per `(strategy, mint)` ledger: `quantity`, `cost_usdc`.

### Capacity accounting

Matches the EVM capacity model:

- `deployable = capacity_usdc - deployed_usdc`
- Buys increase `deployed_usdc` by USDC spent.
- Sells reduce `deployed_usdc` by cost basis sold and adjust `capacity_usdc`:
  - Realized profit restores capacity up to `limit_usdc`.
  - Realized loss reduces capacity.

## Instructions

| Instruction | Signer | Notes |
| --- | --- | --- |
| `InitWallet` | owner | One-time wallet config |
| `InitStrategy` | owner | Create-only; sets `capacity = limit` |
| `SetLimit` | owner | Updates limit/expiry; never clears revocation |
| `RotateSession` | owner | New session pubkey + nonce bump; reactivates |
| `Revoke` | owner or session | Marks strategy revoked |
| `ExecuteSwap` | session + relayer | Relayer pays fees/rent; pinned Jupiter CPI only |
| `WithdrawAsset` | owner | Emergency vault withdrawal |
| `CloseStrategy` | owner | Requires `deployed_usdc == 0` |

### ExecuteSwap

Fixed accounts (in order):

1. session (signer)
2. relayer (signer, writable fee/rent payer)
3. owner
4. wallet
5. strategy
6. vault_authority
7. owner_usdc (owner ATA for funding/refunds and sell proceeds only)
8. strategy_usdc (vault-authority USDC ATA; Jupiter buy source/sell destination)
9. strategy_token_vault (vault-authority ATA for the traded token)
10. asset (StrategyAsset PDA; relayer-created if missing)
11. token_mint
12. usdc_mint
13. token_program — pinned to `spl_token::ID`
14. associated_token_program — pinned to ATA program
15. system_program — pinned
16. program_authority (USDC delegate PDA)
17. jupiter_program — must match wallet config
18. …the selected Jupiter swap instruction’s accounts, in exact order

**Buy:** the program moves at most `usdc_amount` from `owner_usdc` to
`strategy_usdc`, invokes the selected Jupiter swap with the vault-authority PDA signature,
refunds any unused input, and requires measured token output to be at least `token_amount`.
Jupiter cannot access `owner_usdc`.

**Sell:** Jupiter spends directly from `strategy_token_vault` and returns USDC to
`strategy_usdc`. The program requires measured token input to be at most `token_amount`,
requires normalized USDC output to be at least `usdc_amount`, then transfers the measured
proceeds to `owner_usdc`.

Args: `is_buy`, `usdc_amount` (6-decimal max input on buy/min output on sell),
`token_amount` (atomic min output on buy/max input on sell), and the selected Jupiter swap
instruction data. Compute-budget/setup/cleanup instructions are not executed through CPI.

## Build

```bash
cd solana
solana-verify build .
```

Requires Docker and `solana-verify`.

## Recovery

Owner may `Revoke`, `WithdrawAsset`, and `CloseStrategy` through any RPC —
no backend required. Session rotation requires an owner signature.
