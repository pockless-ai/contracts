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
  The CLI source contains the official USDC address or mint for every supported chain, so
  mainnet USDC values are not supplied through `.env`. Preflight validates them on-chain.

### Environment variables

- `*_RPC_URL`: JSON-RPC endpoint for each target chain. Testnet requires
  `BASE_SEPOLIA_RPC_URL` and `SOLANA_DEVNET_RPC_URL`; mainnet requires all listed mainnet
  endpoints.
- `BASE_SEPOLIA_USDC_ADDRESS`: official Base Sepolia USDC contract published by Circle:
  `0x036CbD53842c5426634e7929541eC2318f3dCF7e`. Confirm it in
  [Circle's registry](https://developers.circle.com/stablecoins/usdc-contract-addresses).
- `EVM_FOUNDRY_ACCOUNT`: encrypted Foundry keystore account name.
- `EVM_DEPLOYER_ADDRESS`: public address belonging to that Foundry account.
- `ETHERSCAN_API_KEY`: Etherscan V2 key for explorer source verification.
- `SOLANA_FEE_PAYER_KEYPAIR`: absolute path to the funded Solana keypair JSON. This one
  wallet is both fee payer and upgrade/verification authority.
- `SOLANA_PROGRAM_KEYPAIR`: absolute path to a separate unfunded deployment-only keypair.
  Its public key becomes the program ID and must differ from the fee payer.
- `SOLANA_VERIFY_REPOSITORY_URL`: public source repository used by `solana-verify`.
- Other `SOLANA_VERIFY_*` values control verification polling and the conservative authority
  balance; their checked-in defaults normally need no changes.

Keypair variables contain file paths, never private keys or recovery phrases.

Configure an encrypted Foundry signer using one option.

**Create a dedicated wallet (recommended):**

```bash
mkdir -p "$HOME/.foundry/keystores"
cast wallet new "$HOME/.foundry/keystores" pockless-release
```

**Or import an existing dedicated wallet:**

```bash
cast wallet import pockless-release --interactive
```

Enter the wallet's real private key through the hidden prompt. Do not enter random text.
For either option:

```bash
cast wallet address --account pockless-release
export EVM_FOUNDRY_ACCOUNT=pockless-release
export EVM_DEPLOYER_ADDRESS=0xYourPublicAddress
```

Foundry stores the generated or imported signer in an encrypted keystore and prompts for a
keystore password. Back up the encrypted keystore and password separately. Raw private-key
flags and environment variables are unsupported on every network.

To restore the same signer on another machine, securely copy its encrypted keystore into
`$HOME/.foundry/keystores/`, set mode `600`, and use the original password. Prefer this over
exporting the raw key. If another wallet cannot import the keystore, the last-resort command
`cast wallet private-key --account pockless-release` prints the raw key after a password
prompt. Run it only in a private, unrecorded local terminal and never place its output in
chat, screenshots, `.env`, shell history, or source control.

Restore the Solana keypair from encrypted storage only for the operation:

```bash
chmod 600 /secure/tmp/solana-deployer.json
chmod 600 /secure/tmp/solana-program-id.json
export SOLANA_FEE_PAYER_KEYPAIR=/secure/tmp/solana-deployer.json
export SOLANA_PROGRAM_KEYPAIR=/secure/tmp/solana-program-id.json
```

Only the deployer needs SOL. The separate program-ID keypair is an initial-deployment signer
and must remain unfunded. Mainnet must use production-only keypairs that are never reused
from devnet. Generate them on a trusted encrypted machine, make encrypted offline backups
before deployment, and restore only temporary `chmod 600` copies. Keep the deployer backup
until verification and the separate immutability operation are complete.

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

Set `SOLANA_VERIFY_REPOSITORY_URL` to the public repository. The CLI uses the fee payer as
the upgrade/verification authority, creates an isolated temporary Solana CLI config, and
runs `solana-verify verify-from-repo`
interactively for the exact pushed commit, library `strategy_spend`, and mount path
`solana/programs/strategy-spend`. It then submits the remote job and waits until the
public verification API reports matching hashes. Mainnet fails if the commit is dirty,
unpushed, or not publicly verified.

Uploading verification metadata needs a small extra SOL balance. Preflight adds that amount
to the one deployer check. The default conservative authority minimum and verification
polling timeout can be adjusted with the documented `SOLANA_VERIFY_*` variables in
`deploy/.env.example`.

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
