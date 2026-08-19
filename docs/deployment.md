# Deployment

Deploy the programs, verify bytecode/program hashes, record addresses in
[`deployments.json`](./deployments.json), and publish a release.

**New to deploying?** Start with [`first-deployment.md`](./first-deployment.md).

Nothing in this guide assumes a particular backend or product — only chain tooling and
public RPC endpoints.

## Secure deployment CLI

With Node 20+, Git, cURL, and Yarn installed, bootstrap the contract toolchains:

```bash
yarn setup
```

This installs Foundry, Rust, Solana platform tools (`cargo-build-sbf`), `solana-verify`,
and initializes `forge-std`. Copy `deploy/.env.example` to the gitignored `deploy/.env`, inject the same
variables from a secret manager, or export them in the local shell. RPC URLs and signer
paths are required per target.

Profiles are fixed:

- `testnet`: Base Sepolia (84532) and Solana devnet. Set
  `BASE_SEPOLIA_USDC_ADDRESS`; the CLI will not guess or accept a zero address.
- `mainnet`: Ethereum, Base, Arbitrum, Optimism, Polygon, BNB, and Solana mainnet-beta.
  Canonical USDC addresses/mints are checked into the CLI configuration.

Create an encrypted Foundry signer:

```bash
cast wallet import pockless-release --interactive
export EVM_FOUNDRY_ACCOUNT=pockless-release
export EVM_DEPLOYER_ADDRESS=0xYourPublicAddress
```

Foundry prompts for the keystore password. Raw private-key flags and environment variables
are unsupported on every network. Restore the Solana fee payer, upgrade-authority, and
deterministic program keypairs from encrypted storage only for the operation:

```bash
chmod 600 /secure/tmp/{fee-payer,upgrade-authority,program-id}.json
export SOLANA_FEE_PAYER_KEYPAIR=/secure/tmp/fee-payer.json
export SOLANA_UPGRADE_AUTHORITY_KEYPAIR=/secure/tmp/upgrade-authority.json
export SOLANA_PROGRAM_KEYPAIR=/secure/tmp/program-id.json
```

Run a complete non-signing preflight, fund the reported public addresses manually, then
repeat until every deficit is zero:

```bash
yarn deploy --environment testnet --dry-run
yarn deploy --environment mainnet --dry-run
```

Preflight checks tools, release metadata, tests/builds, RPC chain IDs/genesis hashes,
USDC code and decimals/mint existence, artifact hashes, and balances. EVM gas is estimated
over RPC. Solana reports a deliberately conservative rent estimate based on program size;
it is not presented as an exact deployment quote. The default 20% buffer can be changed
with `--safety-buffer-percent`. No funding or sweeping is automated.

Deploy with the same command minus `--dry-run`. Mainnet requires a clean pushed commit,
an attached TTY, and the exact prompted phrase. `--skip-tests`,
`--skip-solana-verification`, and `--redeploy` are testnet-only. Testnet verification may
be left pending only with the explicit opt-out.

State is atomically saved after every broadcast and failure in
`deploy/.deploy/<environment>.json` (gitignored). Re-running validates completed on-chain
code and resumes receipt/verification work. It never silently replaces a mainnet address.
Successful targets merge into `docs/deployments.json` without deleting other environments.
Back up the completed mainnet manifest with the release records: the later immutability
command requires it at the same path, so it must not exist only on one workstation.

### User delegation (EIP-7702)

Owners delegate their **EOA** to the implementation address (wallet UX varies). Admin
functions (`grant`, `revoke`, `setLimit`, `rotateSession`) execute as self-calls on the
delegated EOA. Swaps use `executeSwap` with a session-signed EIP-712 intent; any funded
relayer may submit.

## Solana — strategy-spend

### Prerequisites

- Solana CLI + `cargo-build-sbf`
- Funded deployer keypair

### Build

```bash
cd solana
cargo build-sbf -- -p strategy-spend
```

### Public Solana verification

Set `SOLANA_VERIFY_REPOSITORY_URL` to the public repository. The CLI derives the uploader
from `SOLANA_UPGRADE_AUTHORITY_KEYPAIR`, creates an isolated temporary Solana CLI config,
and runs `solana-verify verify-from-repo` interactively for the exact pushed commit,
library `strategy_spend`, and mount path `solana/programs/strategy-spend`. It then submits
the remote job and waits until the public verification API reports matching hashes.
Mainnet fails if the commit is dirty, unpushed, or not publicly verified.

Uploading verification metadata requires a small SOL balance on the upgrade-authority
wallet in addition to the deployment fee payer. Preflight reports both deficits. The
default conservative authority minimum and verification polling timeout can be adjusted
with the documented `SOLANA_VERIFY_*` variables in `deploy/.env.example`.

Per wallet, owners run `InitWallet` once, then `InitStrategy` per strategy id.

## deployments.json

Update [`deployments.json`](./deployments.json) with real addresses before publishing.
Recovery web/CLI and integrators should load this file (or a fork with the same shape).

Entries with `status: "not-deployed"` are explicit placeholders, not deployed addresses.

## Release checklist

1. All Foundry tests pass on the release tag.
2. Solana SBF build succeeds; on-chain program hash and public verification are recorded.
3. `deployments.json` matches on-chain addresses.
4. Run `yarn immutable --environment mainnet` only after final review. The command compares
   the verified release hash and current authority, requires a stronger phrase containing
   cluster and program ID, removes authority with `--final`, and reads back `Authority:
none`.
5. Git tag `v*` triggers release workflow → attach `SHA256SUMS`.
6. GitHub Pages serves recovery web from the same tag or `main`.

EVM implementations have no upgrade mechanism and are already immutable. Solana authority
is deliberately retained during deployment and can only be removed by the separate command.
Deploy a new program ID for any future V2; an immutable V1 cannot be upgraded.

## Verification

Independent verifiers should:

1. Check out the release tag.
2. Rebuild EVM bytecode and compare hash to deployed code.
3. Rebuild Solana program and compare to verified program data.
4. Confirm `deployments.json` entries match chain state via RPC.

Integrators should pin to a **release tag + deployments manifest**, not floating `main`.
