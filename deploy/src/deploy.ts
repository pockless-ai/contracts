import { createHash } from "node:crypto"
import {
  access,
  mkdtemp,
  readFile,
  rm,
  stat,
  writeFile,
} from "node:fs/promises"
import { tmpdir } from "node:os"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import {
  createPublicClient,
  encodeDeployData,
  formatEther,
  formatUnits,
  getAddress,
  http,
  keccak256,
  parseAbi,
  type Address,
  type Hex,
} from "viem"
import {
  checked,
  evmDeployArgs,
  runCommand,
  solanaDeployArgs,
  type RunCommand,
} from "./command"
import {
  loadTargets,
  requiredRpc,
  type Environment,
  type EvmTarget,
  type SolanaTarget,
} from "./config"
import { mergeDeployments } from "./deployments"
import { resolveSolanaKeypairs } from "./env"
import {
  assertResumeCompatible,
  loadManifest,
  newManifest,
  saveManifest,
  type DeploymentManifest,
  type TargetState,
} from "./manifest"

const contractsRoot = join(dirname(fileURLToPath(import.meta.url)), "../..")
const evmRoot = join(contractsRoot, "evm")
const solanaRoot = join(contractsRoot, "solana")
const manifestDirectory = join(contractsRoot, "deploy/.deploy")
const deploymentsPath = join(contractsRoot, "docs/deployments.json")
const solanaArtifact = join(solanaRoot, "target/deploy/strategy_spend.so")
const abi = parseAbi(["constructor(address usdcToken_)"])
let derivedFoundryAccount: { account: string; address: Address } | undefined

export type DeployOptions = {
  environment: Environment
  dryRun: boolean
  skipTests: boolean
  skipSolanaVerification: boolean
  redeploy: boolean
  safetyBufferPercent: number
  source: Record<string, string | undefined>
}

export type DeployDependencies = {
  run: RunCommand
  log: (message: string) => void
  setup?: () => Promise<{ releaseCommit: string }>
  preflight?: (
    target: EvmTarget | SolanaTarget
  ) => Promise<{ artifactHash: string }>
  manifestPath?: string
}

function required(source: Record<string, string | undefined>, key: string) {
  const value = source[key]?.trim()
  if (!value) throw new Error(`${key} is required`)
  return value
}

function safeError(error: unknown, source: Record<string, string | undefined>) {
  let message = error instanceof Error ? error.message : String(error)
  for (const value of Object.values(source)) {
    if (value) message = message.split(value).join("<redacted>")
  }
  return message
}

async function sha256(path: string) {
  return createHash("sha256")
    .update(await readFile(path))
    .digest("hex")
}

async function solanaProgramHash(
  run: RunCommand,
  rpc: string,
  programId: string
) {
  const directory = await mkdtemp(join(tmpdir(), "pockless-program-"))
  const output = join(directory, "program.so")
  try {
    await checked(run, "solana", [
      "program",
      "dump",
      programId,
      output,
      "--url",
      rpc,
    ])
    return await sha256(output)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}

async function solanaRpc<T>(
  url: string,
  method: string,
  params: unknown[] = []
) {
  const response = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
  })
  if (!response.ok)
    throw new Error(`Solana RPC ${method} returned HTTP ${response.status}`)
  const payload = (await response.json()) as {
    result?: T
    error?: { message?: string }
  }
  if (payload.error)
    throw new Error(
      `Solana RPC ${method} failed: ${payload.error.message ?? "unknown error"}`
    )
  if (payload.result === undefined)
    throw new Error(`Solana RPC ${method} returned no result`)
  return payload.result
}

async function releaseInfo(run: RunCommand, environment: Environment) {
  const commit = (
    await checked(run, "git", ["rev-parse", "HEAD"], { cwd: contractsRoot })
  ).stdout.trim()
  const dirty = (
    await checked(run, "git", ["status", "--porcelain"], { cwd: contractsRoot })
  ).stdout.trim()
  if (environment === "mainnet" && dirty) {
    throw new Error(
      "mainnet requires a clean release commit; commit or stash all changes"
    )
  }
  if (environment === "mainnet") {
    const upstream = await run(
      "git",
      ["rev-parse", "--abbrev-ref", "@{upstream}"],
      { cwd: contractsRoot }
    )
    if (upstream.code !== 0) {
      throw new Error(
        "mainnet requires HEAD to have a configured pushed upstream"
      )
    }
    const pushed = await run(
      "git",
      ["merge-base", "--is-ancestor", "HEAD", "@{upstream}"],
      {
        cwd: contractsRoot,
      }
    )
    if (pushed.code !== 0) {
      throw new Error(
        "mainnet release commit is not pushed; push HEAD before deployment"
      )
    }
  }
  return commit
}

async function requireTools(run: RunCommand, includeVerify: boolean) {
  const tools = ["git", "forge", "cargo", "solana", "solana-keygen"]
  if (includeVerify) tools.push("solana-verify")
  const unavailable: string[] = []
  for (const tool of tools) {
    try {
      const result = await run(tool, ["--version"])
      if (result.code !== 0) unavailable.push(tool)
    } catch {
      unavailable.push(tool)
    }
  }
  if (unavailable.length > 0) {
    throw new Error(
      `missing required deployment tools: ${unavailable.join(
        ", "
      )}. Install Foundry, Rust/Cargo, Anza Solana CLI, and solana-verify as documented in docs/deployment.md`
    )
  }
}

async function validateKeypair(
  path: string,
  label: string,
  log: (message: string) => void,
  strict: boolean
) {
  await access(path)
  const mode = (await stat(path)).mode & 0o777
  if ((mode & 0o077) !== 0) {
    const message = `${label} permissions are ${mode.toString(8)}; run chmod 600 on the temporary file`
    if (strict) throw new Error(message)
    log(`WARNING: ${message}`)
  }
}

async function withSolanaConfig<T>(
  rpc: string,
  keypair: string,
  operation: (environment: NodeJS.ProcessEnv) => Promise<T>
) {
  const directory = await mkdtemp(join(tmpdir(), "pockless-solana-config-"))
  const path = join(directory, "cli.yml")
  try {
    await writeFile(
      path,
      [
        `json_rpc_url: ${JSON.stringify(rpc)}`,
        "websocket_url: ''",
        `keypair_path: ${JSON.stringify(keypair)}`,
        "address_labels:",
        "commitment: confirmed",
        "",
      ].join("\n"),
      { mode: 0o600 }
    )
    return await operation({ ...process.env, SOLANA_CONFIG_FILE: path })
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}

function positiveInteger(
  source: Record<string, string | undefined>,
  key: string,
  fallback: number
) {
  const value = Number(source[key]?.trim() || fallback)
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`${key} must be a positive integer`)
  }
  return value
}

async function waitForRemoteSolanaVerification(
  programId: string,
  expectedProgramHash: string,
  source: Record<string, string | undefined>,
  log: (message: string) => void
) {
  const statusBase =
    source.SOLANA_VERIFY_STATUS_URL?.trim() || "https://verify.osec.io/status"
  const timeoutMs =
    positiveInteger(source, "SOLANA_VERIFY_TIMEOUT_SECONDS", 1800) * 1000
  const pollMs =
    positiveInteger(source, "SOLANA_VERIFY_POLL_SECONDS", 15) * 1000
  const deadline = Date.now() + timeoutMs
  let lastMessage = "verification is pending"
  while (Date.now() < deadline) {
    const response = await fetch(
      `${statusBase.replace(/\/$/, "")}/${programId}`
    )
    if (response.ok) {
      const result = (await response.json()) as {
        is_verified?: boolean
        message?: string
        on_chain_hash?: string
        executable_hash?: string
      }
      lastMessage = result.message ?? lastMessage
      if (result.is_verified) {
        if (
          result.on_chain_hash &&
          result.executable_hash &&
          result.on_chain_hash !== result.executable_hash
        ) {
          throw new Error(
            "remote Solana verification returned mismatched hashes"
          )
        }
        if (
          result.on_chain_hash &&
          result.on_chain_hash.toLowerCase() !==
            expectedProgramHash.toLowerCase()
        ) {
          throw new Error(
            "remote Solana verification hash does not match the deployed program"
          )
        }
        return
      }
    } else if (response.status !== 404) {
      lastMessage = `verification status returned HTTP ${response.status}`
    }
    log(`Solana public verification pending: ${lastMessage}`)
    await new Promise((resolve) => setTimeout(resolve, pollMs))
  }
  throw new Error(
    `Solana public verification did not complete within ${timeoutMs / 1000} seconds: ${lastMessage}`
  )
}

function artifactBytecode(raw: unknown) {
  const artifact = raw as {
    bytecode?: { object?: Hex }
    deployedBytecode?: { object?: Hex }
  }
  if (!artifact.bytecode?.object || !artifact.deployedBytecode?.object) {
    throw new Error("Foundry artifact is missing bytecode")
  }
  return artifact
}

async function preflightEvm(
  target: EvmTarget,
  options: DeployOptions,
  run: RunCommand,
  log: (message: string) => void,
  existing?: TargetState
) {
  const rpc = requiredRpc(target, options.source)
  const sender = getAddress(required(options.source, "EVM_DEPLOYER_ADDRESS"))
  const account = required(options.source, "EVM_FOUNDRY_ACCOUNT")
  required(options.source, "ETHERSCAN_API_KEY")
  if (!derivedFoundryAccount || derivedFoundryAccount.account !== account) {
    derivedFoundryAccount = {
      account,
      address: getAddress(
        (
          await checked(
            run,
            "cast",
            ["wallet", "address", "--account", account],
            {
              interactive: true,
            }
          )
        ).stdout.trim()
      ),
    }
  }
  const derivedSender = derivedFoundryAccount.address
  if (derivedSender !== sender) {
    throw new Error(
      `${target.name} EVM_DEPLOYER_ADDRESS does not match the Foundry account`
    )
  }
  const client = createPublicClient({ transport: http(rpc) })
  const actualChainId = await client.getChainId()
  if (actualChainId !== target.chainId) {
    throw new Error(
      `${target.name} RPC returned chain id ${actualChainId}, expected ${target.chainId}`
    )
  }
  const tokenCode = await client.getCode({ address: target.usdc })
  if (!tokenCode || tokenCode === "0x")
    throw new Error(`${target.name} USDC has no runtime code`)
  const decimals = await client.readContract({
    address: target.usdc,
    abi: parseAbi(["function decimals() view returns (uint8)"]),
    functionName: "decimals",
  })
  if (decimals !== target.usdcDecimals) {
    throw new Error(
      `${target.name} USDC decimals ${decimals} do not match ${target.usdcDecimals}`
    )
  }
  const artifactPath = join(
    evmRoot,
    "out/SessionSpend7702.sol/SessionSpend7702.json"
  )
  const artifact = artifactBytecode(
    JSON.parse(await readFile(artifactPath, "utf8"))
  )
  const data = encodeDeployData({
    abi,
    bytecode: artifact.bytecode!.object!,
    args: [target.usdc],
  })
  if (!existing?.address || !existing.txHash || options.redeploy) {
    const gas = await client.estimateGas({ account: sender, data })
    const gasPrice = await client.getGasPrice()
    const estimated = gas * gasPrice
    const requiredBalance =
      (estimated * BigInt(100 + options.safetyBufferPercent)) / 100n
    const balance = await client.getBalance({ address: sender })
    const deficit = balance >= requiredBalance ? 0n : requiredBalance - balance
    log(
      `${target.name}: balance=${formatEther(balance)} ETH (${balance} wei), estimated=${formatEther(estimated)} ETH (${estimated} wei), required=${formatEther(requiredBalance)} ETH (${requiredBalance} wei), deficit=${formatEther(deficit)} ETH (${deficit} wei)`
    )
    if (deficit > 0n) {
      throw new Error(
        `${target.name} deployer is underfunded by ${formatEther(deficit)} ETH (${deficit} wei)`
      )
    }
  } else {
    log(
      `${target.name}: deployment transaction is already recorded; funding check skipped`
    )
  }
  return {
    rpc,
    sender,
    account,
    artifactHash: await sha256(artifactPath),
  }
}

async function preflightSolana(
  target: SolanaTarget,
  options: DeployOptions,
  run: RunCommand,
  log: (message: string) => void,
  existing?: TargetState
) {
  const rpc = requiredRpc(target, options.source)
  const { feePayer, authority, programKeypair } = resolveSolanaKeypairs(
    options.source
  )
  log(
    `${target.name}: using SOLANA_FEE_PAYER_KEYPAIR as fee payer and upgrade authority`
  )
  const uniqueKeypairs = [...new Set([feePayer, authority, programKeypair])]
  await Promise.all(
    uniqueKeypairs.map((path) =>
      validateKeypair(
        path,
        path === feePayer
          ? "fee payer"
          : path === authority
            ? "upgrade authority"
            : "program id",
        log,
        options.environment === "mainnet"
      )
    )
  )
  await solanaRpc(rpc, "getVersion")
  const genesis = await solanaRpc<string>(rpc, "getGenesisHash")
  const expectedGenesis =
    target.cluster === "mainnet-beta"
      ? "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d"
      : "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG"
  if (genesis !== expectedGenesis) {
    throw new Error(
      `${target.name} RPC genesis hash does not match ${target.cluster}`
    )
  }
  const mint = await solanaRpc<{ value: unknown | null }>(
    rpc,
    "getAccountInfo",
    [target.usdcMint, { encoding: "base64", commitment: "confirmed" }]
  )
  if (mint.value == null)
    throw new Error(`${target.name} USDC mint does not exist`)
  const payerPubkey = (
    await checked(run, "solana-keygen", ["pubkey", feePayer])
  ).stdout.trim()
  const authorityPubkey = (
    await checked(run, "solana-keygen", ["pubkey", authority])
  ).stdout.trim()
  const programId = (
    await checked(run, "solana-keygen", ["pubkey", programKeypair])
  ).stdout.trim()
  if (programId === payerPubkey) {
    throw new Error(
      "Solana program ID must differ from the fee payer; generate a separate unfunded SOLANA_PROGRAM_KEYPAIR"
    )
  }
  const minimumAuthorityLamports = BigInt(
    positiveInteger(
      options.source,
      "SOLANA_VERIFY_AUTHORITY_MIN_LAMPORTS",
      50_000_000
    )
  )
  if (!existing?.programId || options.redeploy) {
    const balance = await solanaRpc<{ value: number }>(rpc, "getBalance", [
      payerPubkey,
      { commitment: "confirmed" },
    ])
    const lamports = BigInt(balance.value)
    const conservativeSize = (await stat(solanaArtifact)).size * 2 + 4096
    const rent = BigInt(
      await solanaRpc<number>(rpc, "getMinimumBalanceForRentExemption", [
        conservativeSize,
        { commitment: "confirmed" },
      ])
    )
    let requiredBalance =
      (rent * BigInt(100 + options.safetyBufferPercent)) / 100n
    if (!options.skipSolanaVerification) {
      requiredBalance += minimumAuthorityLamports
    }
    const deficit =
      lamports >= requiredBalance ? 0n : requiredBalance - lamports
    log(
      `${target.name}: conservative (not exact) rent estimate=${formatUnits(rent, 9)} SOL (${rent} lamports) for ${conservativeSize} bytes; balance=${formatUnits(lamports, 9)} SOL (${lamports} lamports), required=${formatUnits(requiredBalance, 9)} SOL (${requiredBalance} lamports), deficit=${formatUnits(deficit, 9)} SOL (${deficit} lamports)`
    )
    if (deficit > 0n) {
      throw new Error(
        `${target.name} fee payer is underfunded by ${formatUnits(deficit, 9)} SOL (${deficit} lamports)`
      )
    }
  } else {
    log(
      `${target.name}: deployed program is already recorded; deployment funding check skipped`
    )
  }
  return {
    rpc,
    feePayer,
    authority,
    authorityPubkey,
    programKeypair,
    programId,
    artifactHash: await sha256(solanaArtifact),
  }
}

function parseForgeDeployment(output: string) {
  const parsed = JSON.parse(output) as Record<string, unknown>
  const address = String(parsed.deployedTo ?? parsed.contractAddress ?? "")
  const txHash = String(parsed.transactionHash ?? parsed.transaction_hash ?? "")
  if (
    !/^0x[0-9a-fA-F]{40}$/.test(address) ||
    !/^0x[0-9a-fA-F]{64}$/.test(txHash)
  ) {
    throw new Error(
      "forge did not return a deployment address and transaction hash"
    )
  }
  return { address: getAddress(address), txHash: txHash as Hex }
}

async function deployEvm(
  target: EvmTarget,
  preflight: Awaited<ReturnType<typeof preflightEvm>>,
  options: DeployOptions,
  run: RunCommand,
  onBroadcast: (state: Partial<TargetState>) => Promise<void>,
  existing?: TargetState
): Promise<TargetState> {
  if (
    existing?.artifactHash &&
    existing.artifactHash !== preflight.artifactHash
  ) {
    throw new Error(
      `${target.name} recorded artifact does not match the current release`
    )
  }
  let deployed: { address: Address; txHash: Hex }
  if (existing?.address && existing.txHash && !options.redeploy) {
    deployed = {
      address: getAddress(existing.address),
      txHash: existing.txHash as Hex,
    }
  } else {
    const result = await checked(
      run,
      "forge",
      evmDeployArgs({
        rpc: preflight.rpc,
        account: preflight.account,
        sender: preflight.sender,
        usdc: target.usdc,
      }),
      { cwd: evmRoot, interactive: true }
    )
    deployed = parseForgeDeployment(result.stdout)
    await onBroadcast({
      address: deployed.address,
      txHash: deployed.txHash,
      artifactHash: preflight.artifactHash,
    })
  }
  const client = createPublicClient({ transport: http(preflight.rpc) })
  const receipt = await client.waitForTransactionReceipt({
    hash: deployed.txHash,
  })
  if (receipt.status !== "success")
    throw new Error(`${target.name} deployment reverted`)
  const code = await client.getCode({ address: deployed.address })
  if (!code || code === "0x")
    throw new Error(`${target.name} deployment has no runtime code`)
  const codeHash = keccak256(code)
  const constructorArgs = (
    await checked(run, "cast", [
      "abi-encode",
      "constructor(address)",
      target.usdc,
    ])
  ).stdout.trim()
  await checked(
    run,
    "forge",
    [
      "verify-contract",
      "--root",
      ".",
      "--chain",
      String(target.chainId),
      "--rpc-url",
      preflight.rpc,
      "--verifier",
      "etherscan",
      "--constructor-args",
      constructorArgs,
      "--watch",
      deployed.address,
      "src/SessionSpend7702.sol:SessionSpend7702",
    ],
    { cwd: evmRoot }
  )
  return {
    family: "evm",
    name: target.name,
    status: "complete",
    artifactHash: preflight.artifactHash,
    address: deployed.address,
    txHash: deployed.txHash,
    codeHash,
    verificationStatus: "verified",
    verifiedAt: new Date().toISOString(),
  }
}

async function deploySolana(
  target: SolanaTarget,
  preflight: Awaited<ReturnType<typeof preflightSolana>>,
  options: DeployOptions,
  run: RunCommand,
  log: (message: string) => void,
  onBroadcast: (state: Partial<TargetState>) => Promise<void>,
  existing?: TargetState
): Promise<TargetState> {
  if (
    existing?.artifactHash &&
    existing.artifactHash !== preflight.artifactHash
  ) {
    throw new Error(
      `${target.name} recorded artifact does not match the current release`
    )
  }
  let parsed: { programId?: string; signature?: string }
  if (existing?.programId && !options.redeploy) {
    parsed = {
      programId: existing.programId,
      signature: existing.deploySignature,
    }
  } else {
    const result = await checked(
      run,
      "solana",
      solanaDeployArgs({
        artifact: solanaArtifact,
        rpc: preflight.rpc,
        feePayer: preflight.feePayer,
        authority: preflight.authority,
        programKeypair: preflight.programKeypair,
      })
    )
    parsed = JSON.parse(result.stdout) as {
      programId?: string
      signature?: string
    }
  }
  const programId = parsed.programId ?? preflight.programId
  await onBroadcast({
    programId,
    deploySignature: parsed.signature,
    artifactHash: preflight.artifactHash,
  })
  const shown = JSON.parse(
    (
      await checked(run, "solana", [
        "program",
        "show",
        programId,
        "--url",
        preflight.rpc,
        "--output",
        "json",
      ])
    ).stdout
  ) as {
    programId?: string
    authority?: string | null
    upgradeAuthority?: string | null
  }
  if (shown.programId && shown.programId !== programId) {
    throw new Error(`${target.name} returned a different deployed program id`)
  }
  if (
    (shown.authority ?? shown.upgradeAuthority) !== preflight.authorityPubkey
  ) {
    throw new Error(
      `${target.name} upgrade authority does not match the supplied keypair`
    )
  }
  const programHash = await solanaProgramHash(run, preflight.rpc, programId)
  const repository = options.source.SOLANA_VERIFY_REPOSITORY_URL?.trim()
  let verificationStatus: "verified" | "pending" = "verified"
  let verifiedAt: string | undefined
  if (options.skipSolanaVerification) {
    verificationStatus = "pending"
  } else {
    if (!repository) {
      throw new Error("SOLANA_VERIFY_REPOSITORY_URL is required")
    }
    await withSolanaConfig(
      preflight.rpc,
      preflight.authority,
      async (environment) => {
        await checked(
          run,
          "solana-verify",
          [
            "verify-from-repo",
            "-u",
            preflight.rpc,
            "--program-id",
            programId,
            repository,
            "--commit-hash",
            await releaseInfo(run, options.environment),
            "--library-name",
            "strategy_spend",
            "--mount-path",
            "solana/programs/strategy-spend",
          ],
          { env: environment, interactive: true }
        )
      }
    )
    await checked(run, "solana-verify", [
      "remote",
      "submit-job",
      "--program-id",
      programId,
      "--uploader",
      preflight.authorityPubkey,
    ])
    await waitForRemoteSolanaVerification(
      programId,
      programHash,
      options.source,
      log
    )
    verifiedAt = new Date().toISOString()
  }
  return {
    family: "solana",
    name: target.name,
    status: "complete",
    artifactHash: preflight.artifactHash,
    programId,
    programHash,
    deploySignature: parsed.signature,
    verificationStatus,
    verifiedAt,
  }
}

export async function runDeploy(
  options: DeployOptions,
  dependencies: DeployDependencies = { run: runCommand, log: console.log }
) {
  if (
    options.environment === "mainnet" &&
    (options.skipTests || options.redeploy || options.skipSolanaVerification)
  ) {
    throw new Error(
      "mainnet forbids --skip-tests, --redeploy, and --skip-solana-verification"
    )
  }
  let commit: string
  if (dependencies.setup) {
    commit = (await dependencies.setup()).releaseCommit
  } else {
    await requireTools(dependencies.run, !options.skipSolanaVerification)
    commit = await releaseInfo(dependencies.run, options.environment)
    if (!options.skipTests) {
      await checked(
        dependencies.run,
        "forge",
        ["fmt", "--root", ".", "--check"],
        { cwd: evmRoot }
      )
      await checked(dependencies.run, "forge", ["build", "--root", "."], {
        cwd: evmRoot,
      })
      await checked(dependencies.run, "forge", ["test", "--root", "."], {
        cwd: evmRoot,
      })
      await checked(
        dependencies.run,
        "cargo",
        ["fmt", "--all", "--", "--check"],
        { cwd: solanaRoot }
      )
      await checked(
        dependencies.run,
        "cargo",
        ["test", "-p", "strategy-spend"],
        { cwd: solanaRoot }
      )
      await checked(
        dependencies.run,
        "cargo",
        ["build-sbf", "--", "-p", "strategy-spend"],
        { cwd: solanaRoot }
      )
    } else {
      await access(
        join(evmRoot, "out/SessionSpend7702.sol/SessionSpend7702.json")
      )
      await access(solanaArtifact)
    }
  }
  const targets = loadTargets(options.environment, options.source)
  const path =
    dependencies.manifestPath ??
    join(manifestDirectory, `${options.environment}.json`)
  let manifest = await loadManifest(path)
  if (manifest) assertResumeCompatible(manifest, options.environment, commit)
  else manifest = newManifest(options.environment, commit)

  const preflights = new Map<string, { artifactHash: string }>()
  for (const target of targets) {
    const existing = manifest.targets[target.key]
    try {
      const preflight = dependencies.preflight
        ? await dependencies.preflight(target)
        : target.family === "evm"
          ? await preflightEvm(
              target,
              options,
              dependencies.run,
              dependencies.log,
              existing
            )
          : await preflightSolana(
              target,
              options,
              dependencies.run,
              dependencies.log,
              existing
            )
      preflights.set(target.key, preflight)
      if (
        existing?.status === "complete" &&
        !options.redeploy &&
        !dependencies.preflight
      ) {
        if (target.family === "evm") {
          const evmPreflight = preflight as Awaited<
            ReturnType<typeof preflightEvm>
          >
          if (!existing.address || !existing.codeHash) {
            throw new Error(
              `${target.name} completed manifest is missing address or code hash`
            )
          }
          const client = createPublicClient({
            transport: http(evmPreflight.rpc),
          })
          const code = await client.getCode({
            address: getAddress(existing.address),
          })
          if (!code || keccak256(code) !== existing.codeHash) {
            throw new Error(
              `${target.name} completed deployment no longer matches on-chain code`
            )
          }
        } else {
          const solanaPreflight = preflight as Awaited<
            ReturnType<typeof preflightSolana>
          >
          if (!existing.programId || !existing.programHash) {
            throw new Error(
              `${target.name} completed manifest is missing program id or hash`
            )
          }
          if (
            (await solanaProgramHash(
              dependencies.run,
              solanaPreflight.rpc,
              existing.programId
            )) !== existing.programHash
          ) {
            throw new Error(
              `${target.name} completed deployment no longer matches on-chain program`
            )
          }
        }
        dependencies.log(
          `${target.name}: completed deployment validated; skipping`
        )
      }
    } catch (error) {
      manifest.targets[target.key] = {
        ...(existing ?? { family: target.family, name: target.name }),
        status: "failed",
        error: safeError(error, options.source),
      }
      await saveManifest(path, manifest)
      throw error
    }
  }

  if (options.dryRun) {
    for (const target of targets) {
      const preflight = preflights.get(target.key)!
      if (manifest.targets[target.key]?.status === "complete") continue
      manifest.targets[target.key] = {
        family: target.family,
        name: target.name,
        status: "pending",
        artifactHash: preflight.artifactHash,
      }
      await saveManifest(path, manifest)
    }
    return manifest
  }

  for (const target of targets) {
    const existing = manifest.targets[target.key]
    if (existing?.status === "complete" && !options.redeploy) {
      continue
    }
    if (existing?.status === "complete" && options.environment === "mainnet") {
      throw new Error(`mainnet deployment ${target.name} cannot be overwritten`)
    }
    manifest.targets[target.key] = {
      ...existing,
      family: target.family,
      name: target.name,
      status: "running",
      error: undefined,
    }
    await saveManifest(path, manifest)
    try {
      const preflight = preflights.get(target.key)!
      const persistBroadcast = async (state: Partial<TargetState>) => {
        manifest!.targets[target.key] = {
          ...manifest!.targets[target.key],
          ...state,
        }
        await saveManifest(path, manifest!)
      }
      manifest.targets[target.key] =
        target.family === "evm"
          ? await deployEvm(
              target,
              preflight as Awaited<ReturnType<typeof preflightEvm>>,
              options,
              dependencies.run,
              persistBroadcast,
              existing
            )
          : await deploySolana(
              target,
              preflight as Awaited<ReturnType<typeof preflightSolana>>,
              options,
              dependencies.run,
              dependencies.log,
              persistBroadcast,
              existing
            )
      await saveManifest(path, manifest)
    } catch (error) {
      manifest.targets[target.key] = {
        ...manifest.targets[target.key],
        status: "failed",
        error: safeError(error, options.source),
      }
      await saveManifest(path, manifest)
      throw error
    }
  }
  await mergeDeployments(deploymentsPath, manifest, targets)
  return manifest
}
