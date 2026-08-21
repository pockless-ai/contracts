import {
  encodeSessionGrant,
  encodeSessionRevoke,
  sessionSpend7702Abi,
} from "@pockless/protocol-sdk"
import {
  createPublicClient,
  formatUnits,
  getAddress,
  http,
  keccak256,
  parseAbi,
  toBytes,
  type Address,
  type Hex,
} from "viem"
import { checked, runCommand, type CommandOptions } from "../../../src/command"
import type { SmokeConfig } from "./config"

const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000" as Address
const ZERO_TX_HASH =
  "0x0000000000000000000000000000000000000000000000000000000000000000" as Hex
const EIP7702_GAS_LIMIT = "500000"

function castOptions(config: SmokeConfig["evm"]): CommandOptions {
  return {
    interactive: !config.foundryPassword && Boolean(process.stdin.isTTY),
  }
}

function castAccountArgs(config: SmokeConfig["evm"]) {
  const args = ["--account", config.account]
  if (config.foundryPassword) args.push("--password", config.foundryPassword)
  return args
}

const sessionViewAbi = [
  ...sessionSpend7702Abi,
  {
    type: "function",
    name: "sessionOf",
    stateMutability: "view",
    inputs: [
      { name: "strategyId", type: "bytes32" },
      { name: "key", type: "address" },
    ],
    outputs: [
      {
        name: "",
        type: "tuple",
        components: [
          { name: "limitUsdc", type: "uint128" },
          { name: "capacityUsdc", type: "uint128" },
          { name: "deployedUsdc", type: "uint128" },
          { name: "expiresAt", type: "uint64" },
          { name: "nonce", type: "uint64" },
          { name: "revoked", type: "bool" },
          { name: "exists", type: "bool" },
        ],
      },
    ],
  },
] as const

export function evmClient(config: SmokeConfig["evm"]) {
  return createPublicClient({
    transport: http(config.rpc),
    chain: {
      id: config.chainId,
      name: `chain-${config.chainId}`,
      nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
      rpcUrls: { default: { http: [config.rpc] } },
    },
  })
}

export async function assertEvmDeployment(config: SmokeConfig["evm"]) {
  const client = evmClient(config)
  const chainId = await client.getChainId()
  if (chainId !== config.chainId) {
    throw new Error(`RPC chain id ${chainId} does not match ${config.chainId}`)
  }

  const code = await client.getCode({ address: config.implementation })
  if (!code || code === "0x") {
    throw new Error("SessionSpend7702 implementation has no runtime code")
  }

  const usdcToken = await client.readContract({
    address: config.implementation,
    abi: parseAbi(["function usdcToken() view returns (address)"]),
    functionName: "usdcToken",
  })
  if (getAddress(usdcToken) !== config.usdc) {
    throw new Error("Implementation USDC token does not match testnet config")
  }

  const usdcDecimals = await client.readContract({
    address: config.implementation,
    abi: parseAbi(["function usdcDecimals() view returns (uint8)"]),
    functionName: "usdcDecimals",
  })
  if (usdcDecimals !== 6) {
    throw new Error(`Expected USDC decimals 6, got ${usdcDecimals}`)
  }
}

export async function isEvmDelegated(
  config: SmokeConfig["evm"],
  implementation = config.implementation
) {
  const client = evmClient(config)
  const code = await client.getCode({ address: config.owner })
  return (
    code?.toLowerCase() ===
    `0xef0100${implementation.slice(2).toLowerCase()}`
  )
}

function parseCastTxHash(output: string): Hex {
  const trimmed = output.trim()
  if (/^0x[a-fA-F0-9]{64}$/.test(trimmed) && trimmed !== ZERO_TX_HASH) {
    return trimmed as Hex
  }

  const labeled = output.match(/transactionHash\s+(0x[a-fA-F0-9]{64})/i)
  if (labeled?.[1] && labeled[1] !== ZERO_TX_HASH) {
    return labeled[1] as Hex
  }

  const hashes = [...output.matchAll(/0x[a-fA-F0-9]{64}/g)]
    .map((match) => match[0] as Hex)
    .filter((hash) => hash !== ZERO_TX_HASH)
  if (hashes.length === 0) {
    throw new Error(`Could not parse cast transaction hash from output: ${output}`)
  }
  return hashes.at(-1)!
}

async function waitForCastTx(config: SmokeConfig["evm"], hash: Hex) {
  const receipt = await evmClient(config).waitForTransactionReceipt({
    hash,
    timeout: 120_000,
  })
  if (receipt.status !== "success") {
    throw new Error(`Transaction ${hash} reverted`)
  }
  return hash
}

async function castSendAndWait(
  config: SmokeConfig["evm"],
  args: readonly string[]
): Promise<Hex> {
  const result = await checked(
    runCommand,
    "cast",
    [...args, "--async"],
    castOptions(config)
  )
  const hash = parseCastTxHash(result.stdout)
  return waitForCastTx(config, hash)
}

async function castSelfCall(config: SmokeConfig["evm"], data: Hex) {
  const args = [
    "send",
    config.owner,
    data,
    "--rpc-url",
    config.rpc,
    ...castAccountArgs(config),
  ]
  if (await isEvmDelegated(config)) {
    args.push("--gas-limit", EIP7702_GAS_LIMIT)
  }
  return castSendAndWait(config, args)
}

async function cast7702Send(
  config: SmokeConfig["evm"],
  data: Hex,
  authAddress: Address
) {
  return castSendAndWait(config, [
    "send",
    config.owner,
    data,
    "--rpc-url",
    config.rpc,
    ...castAccountArgs(config),
    "--auth",
    authAddress,
    "--gas-limit",
    EIP7702_GAS_LIMIT,
  ])
}

async function waitForEvmSession(input: {
  config: SmokeConfig["evm"]
  strategyId: Hex
  sessionKey: Address
  predicate: (state: Awaited<ReturnType<typeof readEvmSession>>) => boolean
  timeoutMs?: number
  errorMessage: string
}) {
  const deadline = Date.now() + (input.timeoutMs ?? 60_000)
  while (Date.now() < deadline) {
    const state = await readEvmSession(input)
    if (input.predicate(state)) return state
    await new Promise((resolve) => setTimeout(resolve, 1_000))
  }
  throw new Error(input.errorMessage)
}

async function waitForEvmDelegation(
  config: SmokeConfig["evm"],
  expected: boolean,
  timeoutMs = 60_000
) {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    if ((await isEvmDelegated(config)) === expected) return
    await new Promise((resolve) => setTimeout(resolve, 1_000))
  }
  throw new Error(
    expected
      ? "EIP-7702 delegation did not activate on the owner EOA"
      : "EIP-7702 delegation was not cleared from the owner EOA"
  )
}

export async function ensureEvmDelegation(config: SmokeConfig["evm"]) {
  if (await isEvmDelegated(config)) return
  await cast7702Send(config, "0x", config.implementation)
  await waitForEvmDelegation(config, true)
}

export async function clearEvmDelegation(config: SmokeConfig["evm"]) {
  if (!(await isEvmDelegated(config))) return
  await cast7702Send(config, "0x", ZERO_ADDRESS)
  await waitForEvmDelegation(config, false)
}

export function uniqueStrategyId(label: string): Hex {
  return keccak256(toBytes(`${label}-${Date.now()}-${Math.random()}`))
}

export async function grantEvmSession(input: {
  config: SmokeConfig["evm"]
  strategyId: Hex
  sessionKey: Address
  limitUsdc?: bigint
  expiresAt?: bigint
}) {
  const data = encodeSessionGrant({
    strategyId: input.strategyId,
    sessionAddress: input.sessionKey,
    limitUsdc: formatUnits(input.limitUsdc ?? 1_000_000n, 6),
    expiresAt: Number(
      input.expiresAt ?? BigInt(Math.floor(Date.now() / 1000) + 3600)
    ),
  })
  if (!(await isEvmDelegated(input.config))) {
    const hash = await cast7702Send(
      input.config,
      data,
      input.config.implementation
    )
    await waitForEvmDelegation(input.config, true)
    return hash
  }
  return castSelfCall(input.config, data)
}

export async function readEvmSession(input: {
  config: SmokeConfig["evm"]
  strategyId: Hex
  sessionKey: Address
}) {
  const client = evmClient(input.config)
  return client.readContract({
    address: input.config.owner,
    abi: sessionViewAbi,
    functionName: "sessionOf",
    args: [input.strategyId, input.sessionKey],
  })
}

export async function revokeEvmSession(input: {
  config: SmokeConfig["evm"]
  strategyId: Hex
  sessionKey: Address
}) {
  const data = encodeSessionRevoke({
    strategyId: input.strategyId,
    sessionAddress: input.sessionKey,
  })
  const hash = await castSelfCall(input.config, data)
  await waitForEvmSession({
    config: input.config,
    strategyId: input.strategyId,
    sessionKey: input.sessionKey,
    predicate: (state) => state.revoked === true,
    errorMessage: "EVM session revoke did not appear on-chain",
  })
  return hash
}

export async function cleanupEvmSession(input: {
  config: SmokeConfig["evm"]
  strategyId: Hex
  sessionKey: Address
}) {
  try {
    const state = await readEvmSession(input)
    if (!state.exists || state.revoked) return
    await revokeEvmSession(input)
  } catch {
    // Best-effort cleanup when a smoke run fails mid-lifecycle.
  }
}
