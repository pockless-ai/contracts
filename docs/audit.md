# External audit

The contracts are **not audited yet**. Do not describe them as audited until a firm publishes
a report you link from a release.

## Why audit

These programs hold or move user funds via session keys. An audit looks for:

- Accounting bugs (capacity / deployed / inventory desync)
- Authorization bypass (session acting outside limits)
- Router / CPI escape hatches
- Reentrancy, replay, and signature issues
- Edge cases in limit changes, rotation, and revocation

Internal tests reduce risk but do not replace independent review.

## How to get an audit

### 1. Prepare a package

Auditors need a **frozen scope**:

- Git tag (e.g. `v0.2.0`)
- Exact commit hash
- List of in-scope files (`evm/src/`, `solana/programs/strategy-spend/`)
- [`deployments.json`](./deployments.json) (even if placeholders)
- [`threat-model.md`](./threat-model.md) and [`token-policy.md`](./token-policy.md)
- Test instructions (`forge test`, `cargo test -p strategy-spend`)
- Deployment CLI tests and the exact `deploy/.deploy/<environment>.json` release manifest
- Etherscan V2 and `solana-verify` links tied to the same pushed commit
- Known issues / open questions

### 2. Choose a firm

Contact 2–3 firms for quotes. Examples (not endorsements):

- [Trail of Bits](https://www.trailofbits.com/)
- [OpenZeppelin](https://www.openzeppelin.com/security-audits)
- [Spearbit / Cantina](https://cantina.xyz/)
- [Neodyme](https://neodyme.io/) (Solana-focused)
- [OtterSec](https://osec.io/) (Solana-focused)

Ask for **EVM + Solana** experience, EIP-7702 familiarity, and sample reports.

### 3. Typical process

| Phase         | Duration (typical)            |
| ------------- | ----------------------------- |
| Scoping call  | 1 week                        |
| Audit         | 2–6 weeks depending on scope  |
| Fix + retest  | 1–3 weeks                     |
| Public report | After fixes merged and tagged |

Budget is often **$40k–$150k+** for dual-chain programs of this size — get quotes.

### 4. After the audit

1. Fix findings; tag `v0.2.1` (or next patch).
2. Publish the report in `docs/audit/` or link from the GitHub release.
3. Update README: “Audited by X — report [link] — commit `abc123`”.
4. Optional: [Immunefi bug bounty](https://immunefi.com/) for ongoing disclosure.

### 5. Lighter alternatives (not substitutes)

| Option                                        | Use when                                        |
| --------------------------------------------- | ----------------------------------------------- |
| **Competitive audit** (Code4rena, Sherlock)   | Public contest, smaller budget, fixed timeline  |
| **Review-only** (1–2 senior reviewers)        | Pre-audit sanity check                          |
| **Formal verification** (specific properties) | High-assurance subset (expensive, narrow scope) |

For mainnet with real user limits, plan for a **full audit** of both chains.

## What we do before engaging auditors

- [x] EVM Foundry tests (limits, replay, router policy, grant create-only)
- [x] Solana integration tests (init, set_limit, revoke, rotate, close)
- [x] Full Solana `ExecuteSwap` buy/sell tests with mock Jupiter CPI
- [ ] Public testnet deployment + manual recovery drill
- [ ] Final `deployments.json` on testnet
- [ ] Mainnet dry-run from an attached TTY reports zero funding deficits
- [ ] Solana immutability reviewed as a separate irreversible release operation

Update this checklist before sending the RFP.
