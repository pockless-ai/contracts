# Threat model

Trust boundaries and recovery guarantees for `SessionSpend7702` (EVM) and `strategy-spend`
(Solana). This protocol is designed to be usable **without** any particular backend.

## Assets

| Asset | Location | Owner control |
| --- | --- | --- |
| Wallet native balance | Owner EOA / Solana keypair | Direct signature |
| ERC-20 / SPL tokens | Owner wallet or strategy vaults | Owner admin + recovery tooling |
| Session keys | Held by whoever provisioned them (often an integrator) | Revocable on-chain by owner |
| Off-chain metadata | Optional integrator database | Not required for recovery |

## Trust assumptions

1. **Owner key custody** — Recovery assumes the owner retains their wallet seed or hardware
   signer. Compromise of the owner key is out of scope; an attacker with the owner key can
   revoke sessions and drain funds.
2. **Public RPC** — Indexing and transactions use untrusted RPC providers. Authorization
   depends only on signed transactions, not RPC honesty.
3. **Pinned routers** — Swaps are constrained to pinned 0x AllowanceHolder (EVM) and Jupiter
   (Solana). Recovery tooling does not invoke swap paths.
4. **Verified deployments** — One implementation per EVM chain and one program id on Solana.
   Users must verify addresses in [`deployments.json`](./deployments.json) and release artifacts
   before delegating.

## Session key threats

| Threat | Mitigation |
| --- | --- |
| Session key signs malicious swap | Router allowlist, spend ceilings, nonce replay guard, calldata hash binding |
| Session key persists after user wants to stop | Owner `revoke`; EVM bumps nonce |
| Session key exceeds allocation | `capacityUsdc` / `deployedUsdc` accounting |
| Relayer censorship | Owner can revoke and withdraw without relayer cooperation |

## Recovery tooling threats

| Threat | Mitigation |
| --- | --- |
| Phishing recovery site | Verify GitHub Pages origin; prefer local CLI; compare bytecode to releases |
| Malicious RPC hides positions | Second RPC; read raw account data |
| Supply-chain in npm package | Verify release `SHA256SUMS`; build from tagged source |
| Wrong chain / program id | Pin addresses from `deployments.json`; confirm network in wallet |

## Out of scope

- Availability of any specific trading UI or automation service
- MEV on owner-initiated transfers
- Social engineering of seed phrases
- Bugs in wallet software unrelated to protocol delegation

## Residual risk

Delegated EVM wallets hold tokens at the owner EOA. Revoking sessions stops future automated
swaps but does not remove EIP-7702 delegation; owners should transfer remaining balances and
remove delegation via their wallet when fully exiting.

This document is technical guidance, not legal advice. Autonomous session-key trading may
have regulatory implications depending on jurisdiction and how an integrator operates it.
