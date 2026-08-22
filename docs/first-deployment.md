# First deployment

Step-by-step guide if you have never deployed a contract before. You will deploy **one EVM
implementation per chain** and **one Solana program**, then record addresses in
[`deployments.json`](./deployments.json).

## What you need

| Item            | EVM                                                                 | Solana                                                                       |
| --------------- | ------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Deployer wallet | Encrypted Foundry account funded with each chain’s native gas token | One funded fee-payer/authority keypair plus one unfunded program-ID keypair  |
| Tooling         | [Foundry](https://book.getfoundry.sh/getting-started/installation)  | [Solana CLI](https://solana.com/docs/intro/installation) + `cargo-build-sbf` |
| Token address   | Canonical USDC on that chain                                        | Mainnet USDC mint (or devnet USDC for testing)                               |
| RPC             | Public RPC URL (Alchemy, Infura, chain default)                     | `https://api.mainnet-beta.solana.com` or devnet                              |

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

### 3. Configure an encrypted deployer account

Choose one option.

**Option A — create a dedicated wallet (recommended):**

```bash
mkdir -p "$HOME/.foundry/keystores"
cast wallet new "$HOME/.foundry/keystores" pockless-release
```

**Option B — import an existing dedicated wallet:**

```bash
cast wallet import pockless-release --interactive
```

Enter the wallet's real private key through the hidden prompt. Do not enter random text.

For either option, print the public address:

```bash
cast wallet address --account pockless-release
```

Save it for `EVM_DEPLOYER_ADDRESS`. Back up the encrypted keystore file and its password
separately. Fund this wallet only with the required deployment gas. Never put a raw private
key in `.env`, source control, or terminal arguments. The deployment CLI accepts only the
encrypted Foundry account name and public sender address.

To use the EVM deployer on another machine, install Foundry and copy the encrypted keystore
file into `$HOME/.foundry/keystores/` over a secure channel. Set its mode to `600`; the
original keystore password unlocks it:

```bash
chmod 600 "$HOME/.foundry/keystores/pockless-release"
cast wallet address --account pockless-release
```

Verify that the printed address exactly matches the recorded deployer address before
funding or signing. The keystore file without its password, or the password without the
keystore file, is not sufficient for recovery. If your backup is the raw private key
instead, restore it only through
`cast wallet import pockless-release --interactive`.

The preferred transfer method is copying the encrypted keystore, not exposing the raw key.
If a wallet application cannot import the keystore and you must retrieve the private key,
run this only in a private, unrecorded local terminal:

```bash
cast wallet private-key --account pockless-release
```

The command prompts for the keystore password and prints the raw private key. Import it on
the other device through that wallet's private-key import screen. Treat any terminal or
clipboard history containing it as sensitive. Never paste the key into chat, screenshots,
`.env`, shell history, or source control. `cast wallet new` does not provide a mnemonic
recovery phrase; preserving the encrypted keystore and password is therefore the normal
recovery method.

### 4. Record the EVM configuration

You will create shared `deploy/.env` and environment-specific deployment profiles after
completing the Solana setup below. Record the Foundry account name, its public address, an
Etherscan API key, and the target EVM RPC URL. For
mainnet, the deployment CLI already
contains the official USDC contract address for every supported EVM chain, so you do not
provide mainnet USDC addresses in `.env`. The CLI validates each configured contract on-chain
before deploying. USDC is issued by Circle. For Base Sepolia, use Circle's published address
`0x036CbD53842c5426634e7929541eC2318f3dCF7e` and confirm it in
[Circle's USDC contract registry](https://developers.circle.com/stablecoins/usdc-contract-addresses).
Never use a zero or guessed address.

---

## Part 2 — Solana (strategy-spend)

### 1. Verify the installed Solana toolchain

```bash
solana --version
cargo build-sbf --version
solana-verify --version
```

### 2. Create or restore the signer files

For a first testnet deployment, create a funded deployer and a separate unfunded program-ID
keypair. The deployer is both fee payer and upgrade/verification authority:

```bash
mkdir -p "$HOME/.config/pockless/testnet"
solana-keygen new --outfile "$HOME/.config/pockless/testnet/fee-payer.json"
solana-keygen new --outfile "$HOME/.config/pockless/testnet/program-id.json"
chmod 600 "$HOME/.config/pockless/testnet/"*.json
```

Run each `solana-keygen new` command separately because each prompts interactively.
At `For added security, enter a BIP39 passphrase`, either:

- Press Enter to use no additional seed passphrase for a disposable testnet key; or
- Enter a strong passphrase and back it up separately. Recovery then requires both the seed
  phrase and this passphrase.

The BIP39 passphrase does not encrypt the generated JSON file. The file contains raw signing
material, which is why it must remain on encrypted storage with mode `600`.

Back up both recovery phrases securely. Never commit, upload, or share the JSON files. For
an existing deployment identity, restore the same files from encrypted storage instead of
generating replacements.

To use the signer on another machine, copy its JSON file from encrypted backup over a secure
channel, then run `chmod 600 <path>`. Alternatively, recover it from its seed phrase and
optional BIP39 passphrase:

```bash
RECOVERY_DIR="$HOME/.config/pockless/recovery"
mkdir -p "$RECOVERY_DIR"
chmod 700 "$RECOVERY_DIR"
solana-keygen recover 'prompt:?key=0/0' --outfile "$RECOVERY_DIR/recovered.json"
chmod 600 "$RECOVERY_DIR/recovered.json"
solana-keygen pubkey "$RECOVERY_DIR/recovered.json"
```

Enter recovery material only into the hidden local prompt. Verify that the printed public
key exactly matches the previously recorded deployer address.

For mainnet, create different production-only keypairs on a trusted, encrypted machine.
Never reuse the testnet keys:

```bash
MAINNET_KEY_DIR="$HOME/.config/pockless/mainnet"
mkdir -p "$MAINNET_KEY_DIR"
chmod 700 "$MAINNET_KEY_DIR"
solana-keygen new --outfile "$MAINNET_KEY_DIR/solana-deployer.json"
solana-keygen new --outfile "$MAINNET_KEY_DIR/solana-program-id.json"
chmod 600 \
  "$MAINNET_KEY_DIR/solana-deployer.json" \
  "$MAINNET_KEY_DIR/solana-program-id.json"
```

On macOS, use this home-directory location only when FileVault is enabled. Otherwise, set
`MAINNET_KEY_DIR` to a writable encrypted volume. `/secure/tmp` is not a standard macOS
directory and the sealed system volume prevents creating it.

Before funding or deploying, make encrypted offline backups of both files and recovery
phrases. A BIP39 passphrase protects the recovery phrase but does not encrypt the JSON
keypair file. Restore keypairs only for deployment, verification, recovery, or immutability
operations; remove temporary copies afterward. Only the deployer needs SOL for program rent
plus the small verification balance reported by preflight. Do not fund the program ID.

Set `SOLANA_FEE_PAYER_KEYPAIR` and `SOLANA_PROGRAM_KEYPAIR` to their absolute paths.

### 3. Test and build the program

```bash
cd solana
cargo test --locked -p strategy-spend
solana-verify build .
cd ..
```

Output: `solana/target/deploy/strategy_spend.so`

---

## Part 3 — Deploy EVM and Solana together

The deployment command always processes every target in the selected environment. Do not
run it until both Part 1 and Part 2 are complete.

### 1. Create the environment file

```bash
cp deploy/.env.example deploy/.env
cp deploy/.env.testnet.example deploy/.env.testnet
cp deploy/.env.mainnet.example deploy/.env.mainnet
```

Put shared verification values in `deploy/.env`. Put testnet RPCs and persistent testnet-only
EVM and Solana signer values in `deploy/.env.testnet`. Put mainnet RPCs and persistent
production-only EVM and Solana signer values in `deploy/.env.mainnet`. The CLI automatically
selects the matching profile from `--environment`, so switching environments does not
require editing or copying signer values.

For testnet, fill in:

- `BASE_SEPOLIA_RPC_URL`: Base Sepolia JSON-RPC endpoint from your RPC provider, or the
  rate-limited public endpoint `https://sepolia.base.org`.
- `BASE_SEPOLIA_USDC_ADDRESS`: Circle's official Base Sepolia USDC contract,
  `0x036CbD53842c5426634e7929541eC2318f3dCF7e`.
- `SOLANA_DEVNET_RPC_URL`: Solana devnet RPC endpoint from your provider, or the rate-limited
  public endpoint `https://api.devnet.solana.com`.
- `EVM_FOUNDRY_ACCOUNT`: encrypted Foundry account name, for example `pockless-release`.
- `EVM_DEPLOYER_ADDRESS`: public `0x` address printed by
  `cast wallet address --account pockless-release`.
- `ETHERSCAN_API_KEY`: Etherscan V2 API key used to publish and verify the EVM source.
- `SOLANA_FEE_PAYER_KEYPAIR`: absolute path to the funded Solana keypair JSON. The CLI uses
  it as both fee payer and upgrade/verification authority.
- `SOLANA_PROGRAM_KEYPAIR`: absolute path to the separate unfunded program-ID keypair.
- `SOLANA_VERIFY_REPOSITORY_URL`: public source repository,
  `https://github.com/pockless-ai/contracts`.

The `SOLANA_VERIFY_*` status, timeout, polling, and minimum-balance values already have
defaults in `.env.example`; normally leave them unchanged.

All three populated environment files are gitignored. Never put private keys or recovery
phrases in them; keypair variables contain paths only.

### 2. Fund the testnet signers

An EVM private key produces the same address on Base Sepolia and Base mainnet, and a Solana
keypair produces the same public key on devnet and mainnet-beta. Their balances are separate:
testnet ETH and devnet SOL have no monetary value and cannot be moved to mainnet. Use separate
production-only keys for mainnet even though key reuse is technically possible.

For Base Sepolia, request test ETH from a provider in
[Base's official faucet directory](https://docs.base.org/base-chain/network-information/network-faucets)
and send it to `EVM_DEPLOYER_ADDRESS`. Some providers, including Alchemy, require mainnet
wallet history; use another listed faucet rather than funding mainnet solely to meet faucet
eligibility.

If a faucet requires browser-wallet authentication, connect a separate everyday browser
wallet, claim Base Sepolia ETH to that wallet, and then send the test ETH to
`EVM_DEPLOYER_ADDRESS`. This avoids exposing the dedicated deployer key to a browser
extension. Import the deployer into MetaMask or another provider only for a disposable
testnet identity, never for the production deployer.

For Solana devnet, print and fund the one deployer address:

```bash
solana-keygen pubkey "$HOME/.config/pockless/testnet/fee-payer.json"
solana airdrop 2 "$(solana-keygen pubkey "$HOME/.config/pockless/testnet/fee-payer.json")" --url devnet
```

Public faucets are rate-limited, so retry later or use your provider's faucet if necessary.
That one address covers deployment rent and the small verification balance. Deployment
itself does not require USDC; test USDC is only needed later when testing wallet
authorization and swaps.

Mainnet funding uses real native assets: ETH or the chain's gas token for every selected EVM
network, and SOL for the Solana deployer. The preflight reports the required amount for
each public address before any transaction is signed.

### 3. Run the non-signing preflight

```bash
yarn deploy --environment testnet --dry-run
```

Preflight tests and builds both contracts, validates RPC networks and USDC, verifies signer
identities, and reports funding deficits. Fund only the displayed public addresses, then
repeat the dry-run until every deficit is zero.

### 4. Deploy

```bash
yarn deploy --environment testnet
```

The CLI deploys Base Sepolia and Solana devnet sequentially, verifies them, and writes a
resumable gitignored manifest. EVM verification uses Etherscan V2. Solana verification uses
the exact public Git commit. Testnet may explicitly use `--skip-solana-verification`;
mainnet may not.

Successful targets are merged into [`deployments.json`](./deployments.json) automatically.
Do not hand-copy addresses from terminal output.

Future releases use `yarn upgrade --environment testnet|mainnet`. EVM upgrades create a new
implementation address and require wallet re-delegation; Solana upgrades retain the program
ID until its upgrade authority is permanently removed.

### 5. Make Solana immutable (mainnet only)

Mainnet deployment retains upgrade authority. Only after public verification, hash review,
and final sign-off run:

```bash
yarn immutable --environment mainnet
```

The stronger confirmation includes the cluster and program ID. The CLI verifies the
on-chain hash and authority before removal and reads back no authority afterward. This is
irreversible and cannot be combined with deploy. A future V2 requires a new program ID.

### 6. Activate user wallets (not done at deploy time)

For EVM, each wallet owner delegates their EOA to the implementation address using their
wallet’s EIP-7702 flow. Admin calls are self-calls to the owner address; swaps use
session-signed EIP-712 intents submitted by a relayer.

For Solana, each owner runs:

1. `InitWallet` — pins USDC mint and Jupiter program id.
2. Approve the program’s USDC delegate PDA on their USDC ATA (SPL `approve`).
3. `InitStrategy` per strategy — session key + limit + expiry.

Your app prepares and submits these transactions when a user authorizes.

---

## Part 4 — Release and GitHub Pages

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
