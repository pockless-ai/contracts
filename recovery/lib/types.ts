import type { Address, Hex } from "viem"

export type EvmDeployment = {
  name: string
  rpc: string
  usdc: Address
  implementation: Address
  status?: "not-deployed" | "deployed"
  tier?: "testnet" | "mainnet"
  chainId?: number
  txHash?: Hex
  codeHash?: Hex
  verifiedAt?: string
  releaseCommit?: string
}

export type SolanaDeployment = {
  name: string
  rpc: string
  programId: string
  status?: "not-deployed" | "deployed" | "immutable"
  usdcMint: string
  tier?: "testnet" | "mainnet"
  programHash?: string
  verifiedAt?: string
  immutableAt?: string
  releaseCommit?: string
}

export type Deployments = {
  evm: Record<string, EvmDeployment>
  solana: Record<string, SolanaDeployment>
}

export type EvmSession = {
  strategyId: Hex
  sessionKey: Address
  limitUsdc: bigint
  capacityUsdc: bigint
  deployedUsdc: bigint
  expiresAt: bigint
  nonce: bigint
  revoked: boolean
}

export type SolanaStrategy = {
  pubkey: string
  strategyId: Hex
  session: string
  limitUsdc: bigint
  capacityUsdc: bigint
  deployedUsdc: bigint
  expiresAt: bigint
  nonce: bigint
  revoked: boolean
}

export function hexStrategyId(bytes: Uint8Array): Hex {
  return `0x${Buffer.from(bytes).toString("hex")}` as Hex
}

export function parseStrategyId(value: string): Uint8Array {
  const hex = value.startsWith("0x") ? value.slice(2) : value
  if (hex.length !== 64) {
    throw new Error("strategy-id must be 32 bytes (64 hex chars)")
  }
  return Buffer.from(hex, "hex")
}

export function formatUsdc(amount: bigint): string {
  const whole = amount / 1_000_000n
  const frac = amount % 1_000_000n
  return `${whole}.${frac.toString().padStart(6, "0")}`
}
