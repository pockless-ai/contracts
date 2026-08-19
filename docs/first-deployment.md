# First deployment

Step-by-step guide if you have never deployed a contract before. You will deploy **one EVM
implementation per chain** and **one Solana program**, then record addresses in
[`deployments.json`](./deployments.json).

## What you need

| Item            | EVM                                                                 | Solana                                                                                                            |
| --------------- | ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Deployer wallet | Encrypted Foundry account funded with each chain’s native gas token | Separate fee-payer, upgrade-authority, and deterministic program-ID keypair files; preflight reports required SOL |
| Tooling         | [Foundry](https://book.getfoundry.sh/getting-started/installation)  | [Solana CLI](https://solana.com/docs/intro/installation) + `cargo-build-sbf`                                      |
| Token address   | Canonical USDC on that chain                                        | Mainnet USDC mint (or devnet USDC for testing)                                                                    |
| RPC             | Public RPC URL (Alchemy, Infura, chain default)                     | `https://api.mainnet-beta.solana.com` or devnet                                                                   |

Use **devnet / testnet first**. Do not deploy to mainnet until tests pass and you have an
audit or explicit risk acceptance.

---

## Part 1 — EVM (SessionSpend7702)

### 1. Clone and install toolchains

```bash
git clone https://github.com/pockless-ai/contracts.git
cd contracts
yarn setup
```

This installs Foundry, Rust/Cargo, Solana platform tools, `solana-verify`, and
initializes `forge-std`.

### 2. Test

```bash
cd evm
forge test --root .
cd ..
```

All tests must pass before you deploy.

### 3. Import an encrypted deployer account

```bash
cast wallet import pockless-release --interactive
```

Store the recovery material offline. The deployment CLI accepts only the encrypted Foundry
account name and its public sender address; it never accepts a raw private key.

### 4. Configure the environment

Copy the variable names in `deploy/.env.example`. Mainnet USDC addresses are pinned in the
checked-in profile. Base Sepolia USDC is intentionally required as
`BASE_SEPOLIA_USDC_ADDRESS`; obtain it from the issuer and never use a zero or guessed
address.

### 5. Deploy

```bash
yarn deploy --environment testnet --dry-run
# Fund only the public addresses and rerun preflight until deficits are zero.
yarn deploy --environment testnet
```

The CLI deploys Base Sepolia and Solana devnet sequentially, verifies them, and writes a
resumable gitignored manifest.

### 6. Verification

EVM verification uses Etherscan V2 with `ETHERSCAN_API_KEY`. Solana uses
`solana-verify` against the exact public commit. Testnet may explicitly use
`--skip-solana-verification`; mainnet may not.

### 7. Record the deployment

Successful targets are merged into [`deployments.json`](./deployments.json) automatically.
Do not hand-copy addresses from terminal output.

### 8. How users delegate (EIP-7702)

Deployment alone does not activate anything. Each **wallet owner** must delegate their EOA
to your implementation address using their wallet’s EIP-7702 flow. After delegation:

- Owner admin calls (`grant`, `setLimit`, `rotateSession`, `revoke`) are **self-calls** to
  their own address.
- Swaps use `executeSwap` with a session-signed intent; your relayer submits and pays gas.

Your product (or docs) must guide owners through delegation + `grant`.

---

## Part 2 — Solana (strategy-spend)

### 1. Verify the installed Solana toolchain

```bash
solana --version
cargo build-sbf --version
solana-verify --version
```

### 2. Restore temporary signer files

```bash
chmod 600 /secure/tmp/{fee-payer,upgrade-authority,program-id}.json
export SOLANA_FEE_PAYER_KEYPAIR=/secure/tmp/fee-payer.json
export SOLANA_UPGRADE_AUTHORITY_KEYPAIR=/secure/tmp/upgrade-authority.json
export SOLANA_PROGRAM_KEYPAIR=/secure/tmp/program-id.json
```

### 3. Build the program

```bash
cd solana
cargo build-sbf -- -p strategy-spend
```

Output: `target/deploy/strategy_spend.so`

### 4. Deploy

The same `yarn deploy --environment testnet|mainnet` command deploys Solana with explicit
RPC, fee payer, upgrade authority, and deterministic program-ID keypair paths.

### 5. Make it immutable (mainnet)

Mainnet deployment retains upgrade authority. Only after public verification, hash review,
and final sign-off run:

```bash
yarn immutable --environment mainnet
```

The stronger confirmation includes the cluster and program ID. The CLI verifies the
on-chain hash and authority before removal and reads back no authority afterward. This is
irreversible and cannot be combined with deploy. A future V2 requires a new program ID.

### 6. Record the deployment

The CLI records program ID, deployment signature, hash, verification time, and immutable
time in the manifests.

### 7. Per-wallet setup (not done at deploy time)

Each owner runs **once**:

1. `InitWallet` — pins USDC mint and Jupiter program id.
2. Approve the program’s USDC delegate PDA on their USDC ATA (SPL `approve`).
3. `InitStrategy` per strategy — session key + limit + expiry.

Your app prepares and submits these transactions when a user authorizes.

---

## Part 3 — Release and GitHub Pages

1. Tag a release: `git tag v0.2.0 && git push origin v0.2.0`
2. CI attaches build artifacts and `SHA256SUMS` (see `.github/workflows/release.yml`).
3. Enable GitHub Pages for the recovery web (Settings → Pages → GitHub Actions).

Users should pin integrator config to a **tag + deployments.json**, not floating `main`.

---

## Checklist before mainnet

- [ ] `forge test` passes on the release tag
- [ ] `cargo test -p strategy-spend` passes
- [ ] Bytecode / program hash documented in release notes
- [ ] `deployments.json` matches on-chain addresses
- [ ] Recovery web + CLI tested against devnet
- [ ] External audit complete **or** explicit internal sign-off with documented residual risk
- [ ] Solana upgrade authority removed on mainnet program

See also [`audit.md`](./audit.md), [`e2e-testing.md`](./e2e-testing.md), [`token-policy.md`](./token-policy.md).
