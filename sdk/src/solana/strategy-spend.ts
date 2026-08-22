import { PublicKey } from "@solana/web3.js"
import { keccak256, toBytes, type Hex } from "viem"

export const WALLET_SEED = Buffer.from("wallet")
export const STRATEGY_SEED = Buffer.from("strategy")
export const VAULT_SEED = Buffer.from("vault")
export const ASSET_SEED = Buffer.from("asset")
export const AUTHORITY_SEED = Buffer.from("authority")

/** Borsh enum variant indices — must match `StrategySpendInstruction` in the program. */
const VARIANT = {
  InitWallet: 0,
  InitStrategy: 1,
  SetLimit: 2,
  RotateSession: 3,
  Revoke: 4,
  ExecuteSwap: 5,
  ExecuteSwapWithFees: 6,
  WithdrawAsset: 7,
  CloseStrategy: 8,
} as const

export function solanaStrategyIdFromCuid(strategyCuid: string): Uint8Array {
  const hash = keccak256(toBytes(strategyCuid))
  return Buffer.from(hash.slice(2), "hex")
}

export function walletPda(programId: PublicKey, owner: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [WALLET_SEED, owner.toBuffer()],
    programId
  )
}

export function strategyPda(
  programId: PublicKey,
  owner: PublicKey,
  strategyId: Uint8Array
) {
  return PublicKey.findProgramAddressSync(
    [STRATEGY_SEED, owner.toBuffer(), Buffer.from(strategyId)],
    programId
  )
}

export function vaultPda(programId: PublicKey, strategy: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [VAULT_SEED, strategy.toBuffer()],
    programId
  )
}

export function assetPda(
  programId: PublicKey,
  strategy: PublicKey,
  mint: PublicKey
) {
  return PublicKey.findProgramAddressSync(
    [ASSET_SEED, strategy.toBuffer(), mint.toBuffer()],
    programId
  )
}

export function authorityPda(programId: PublicKey, owner: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [AUTHORITY_SEED, owner.toBuffer()],
    programId
  )
}

/** InitWallet — variant only; mints are passed as accounts. */
export function encodeInitWallet() {
  return Buffer.from([VARIANT.InitWallet])
}

export function encodeInitStrategy(input: {
  strategyId: Uint8Array
  session: PublicKey
  limitUsdc: bigint
  expiresAt: bigint
}) {
  const data = Buffer.alloc(1 + 32 + 32 + 8 + 8)
  data[0] = VARIANT.InitStrategy
  data.set(Buffer.from(input.strategyId), 1)
  data.set(input.session.toBytes(), 33)
  data.writeBigUInt64LE(input.limitUsdc, 65)
  data.writeBigInt64LE(input.expiresAt, 73)
  return data
}

export function encodeSetLimit(input: {
  limitUsdc: bigint
  expiresAt: bigint
}) {
  const data = Buffer.alloc(1 + 8 + 8)
  data[0] = VARIANT.SetLimit
  data.writeBigUInt64LE(input.limitUsdc, 1)
  data.writeBigInt64LE(input.expiresAt, 9)
  return data
}

export function encodeRotateSession(newSession: PublicKey) {
  const data = Buffer.alloc(1 + 32)
  data[0] = VARIANT.RotateSession
  data.set(newSession.toBytes(), 1)
  return data
}

export function encodeRevoke() {
  return Buffer.from([VARIANT.Revoke])
}

export function encodeExecuteSwap(input: {
  isBuy: boolean
  usdcAmount: bigint
  tokenAmount: bigint
  jupiterData: Buffer
}) {
  const data = Buffer.alloc(1 + 1 + 8 + 8 + 4 + input.jupiterData.length)
  data[0] = VARIANT.ExecuteSwap
  data[1] = input.isBuy ? 1 : 0
  data.writeBigUInt64LE(input.usdcAmount, 2)
  data.writeBigUInt64LE(input.tokenAmount, 10)
  data.writeUInt32LE(input.jupiterData.length, 18)
  input.jupiterData.copy(data, 22)
  return data
}

export function encodeExecuteSwapWithFees(input: {
  isBuy: boolean
  usdcAmount: bigint
  tokenAmount: bigint
  platformFeeUsdc: bigint
  gasReimburseUsdc: bigint
  minNativeOut: bigint
  treasury: PublicKey
  jupiterData: Buffer
  gasJupiterData: Buffer
  gasJupiterAccountCount: number
}) {
  const hasGas = input.gasReimburseUsdc > 0n
  if (
    (!hasGas &&
      (input.minNativeOut !== 0n ||
        input.gasJupiterData.length !== 0 ||
        input.gasJupiterAccountCount !== 0)) ||
    (hasGas &&
      (input.minNativeOut <= 0n ||
        input.gasJupiterData.length === 0 ||
        input.gasJupiterAccountCount <= 0 ||
        input.gasJupiterAccountCount > 255))
  ) {
    throw new Error("Invalid gas reimbursement encoding.")
  }
  const gasPayload = hasGas
    ? Buffer.concat([
        Buffer.from([input.gasJupiterAccountCount]),
        input.gasJupiterData,
      ])
    : Buffer.alloc(0)

  const data = Buffer.alloc(
    1 + 1 + 8 + 8 + 8 + 8 + 8 + 32 + 4 + input.jupiterData.length + 4 + gasPayload.length
  )
  let offset = 0
  data[offset++] = VARIANT.ExecuteSwapWithFees
  data[offset++] = input.isBuy ? 1 : 0
  data.writeBigUInt64LE(input.usdcAmount, offset)
  offset += 8
  data.writeBigUInt64LE(input.tokenAmount, offset)
  offset += 8
  data.writeBigUInt64LE(input.platformFeeUsdc, offset)
  offset += 8
  data.writeBigUInt64LE(input.gasReimburseUsdc, offset)
  offset += 8
  data.writeBigUInt64LE(input.minNativeOut, offset)
  offset += 8
  data.set(input.treasury.toBytes(), offset)
  offset += 32
  data.writeUInt32LE(input.jupiterData.length, offset)
  offset += 4
  input.jupiterData.copy(data, offset)
  offset += input.jupiterData.length
  data.writeUInt32LE(gasPayload.length, offset)
  offset += 4
  gasPayload.copy(data, offset)
  return data
}

export function encodeWithdrawAsset(amount: bigint) {
  const data = Buffer.alloc(1 + 8)
  data[0] = VARIANT.WithdrawAsset
  data.writeBigUInt64LE(amount, 1)
  return data
}

export function encodeCloseStrategy() {
  return Buffer.from([VARIANT.CloseStrategy])
}

export type { Hex }
