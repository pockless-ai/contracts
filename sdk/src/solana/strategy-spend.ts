import { PublicKey } from "@solana/web3.js"
import { keccak256, toBytes, type Hex } from "viem"

export const WALLET_SEED = Buffer.from("wallet")
export const STRATEGY_SEED = Buffer.from("strategy")
export const VAULT_SEED = Buffer.from("vault")
export const ASSET_SEED = Buffer.from("asset")
export const AUTHORITY_SEED = Buffer.from("authority")
export const RELAY_RECEIPT_SEED = Buffer.from("relay-receipt")
export const RELAY_PENDING_DEPOSIT_SEED = Buffer.from("relay-pending-deposit")
export const RELAY_PENDING_SELL_SEED = Buffer.from("relay-pending-sell")
export const RELAY_PENDING_GAS_SEED = Buffer.from("relay-pending-gas")
export const REMOTE_ASSET_SEED = Buffer.from("remote-asset")
export const REMOTE_AGGREGATE_SEED = Buffer.from("remote-aggregate")
export const NATIVE_VAULT_SEED = Buffer.from("native-vault")

/** Borsh enum variant indices — must match `StrategySpendInstruction` in the program. */
const VARIANT = {
  InitWallet: 0,
  InitStrategy: 1,
  SetLimit: 2,
  RotateSession: 3,
  Revoke: 4,
  WithdrawAsset: 5,
  CloseStrategy: 6,
  ExecuteSwapWithFees: 7,
  ExecuteRelayDeposit: 8,
  CreditRelayAsset: 9,
  ExecuteRemoteRelaySell: 10,
  CreditUsdcReturn: 11,
  ReleaseRelayDeposit: 12,
  RestoreRemoteRelayAsset: 13,
  ExecuteRelayGasTopUp: 14,
  ReleaseRelayGasTopUp: 15,
} as const

export const RelayAction = {
  Deposit: 0,
  CreditAsset: 1,
  RemoteSell: 2,
  UsdcReturn: 3,
  DepositRelease: 4,
  AssetRestore: 5,
  GasTopUp: 6,
  GasTopUpRelease: 7,
} as const

export type RelayAction = (typeof RelayAction)[keyof typeof RelayAction]

export type SolanaGasMode =
  | "none"
  | "credit_only"
  | "separate"
  | "native_output"

const GAS_MODE: Record<SolanaGasMode, number> = {
  none: 0,
  credit_only: 1,
  separate: 2,
  native_output: 3,
}

/** Accounts passed to InitWallet (instruction data is variant-only). */
export type InitWalletAccounts = {
  relayDepositoryProgram: PublicKey
  platformRelayer: PublicKey
}

export function solanaStrategyIdFromCuid(strategyCuid: string): Uint8Array {
  const hash = keccak256(toBytes(strategyCuid))
  return Buffer.from(hash.slice(2), "hex")
}

function fundingChainSeed(fundingChainId: bigint): Buffer {
  const seed = Buffer.alloc(8)
  seed.writeBigUInt64LE(fundingChainId)
  return seed
}

function appendVecU8(data: Buffer, value: Buffer): Buffer {
  const length = Buffer.alloc(4)
  length.writeUInt32LE(value.length)
  return Buffer.concat([data, length, value])
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

export function relayReceiptPda(
  programId: PublicKey,
  strategy: PublicKey,
  relayOrderId: Uint8Array,
  action: RelayAction
) {
  return PublicKey.findProgramAddressSync(
    [
      RELAY_RECEIPT_SEED,
      strategy.toBuffer(),
      Buffer.from(relayOrderId),
      Buffer.from([action]),
    ],
    programId
  )
}

export function relayPendingDepositPda(
  programId: PublicKey,
  strategy: PublicKey,
  relayOrderId: Uint8Array
) {
  return PublicKey.findProgramAddressSync(
    [
      RELAY_PENDING_DEPOSIT_SEED,
      strategy.toBuffer(),
      Buffer.from(relayOrderId),
    ],
    programId
  )
}

export function relayPendingSellPda(
  programId: PublicKey,
  strategy: PublicKey,
  relayOrderId: Uint8Array
) {
  return PublicKey.findProgramAddressSync(
    [RELAY_PENDING_SELL_SEED, strategy.toBuffer(), Buffer.from(relayOrderId)],
    programId
  )
}

export function relayPendingGasTopUpPda(
  programId: PublicKey,
  strategy: PublicKey,
  relayOrderId: Uint8Array
) {
  return PublicKey.findProgramAddressSync(
    [RELAY_PENDING_GAS_SEED, strategy.toBuffer(), Buffer.from(relayOrderId)],
    programId
  )
}

export function remoteAssetPda(
  programId: PublicKey,
  strategy: PublicKey,
  mint: PublicKey,
  fundingChainId: bigint
) {
  return PublicKey.findProgramAddressSync(
    [
      REMOTE_ASSET_SEED,
      strategy.toBuffer(),
      mint.toBuffer(),
      fundingChainSeed(fundingChainId),
    ],
    programId
  )
}

export function remoteAggregatePda(
  programId: PublicKey,
  strategy: PublicKey,
  mint: PublicKey
) {
  return PublicKey.findProgramAddressSync(
    [REMOTE_AGGREGATE_SEED, strategy.toBuffer(), mint.toBuffer()],
    programId
  )
}

export function nativeVaultPda(programId: PublicKey, strategy: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [NATIVE_VAULT_SEED, strategy.toBuffer()],
    programId
  )
}

/** InitWallet — variant only; relay depository and platform relayer are passed as accounts. */
export function encodeInitWallet(_accounts?: InitWalletAccounts) {
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

export function encodeExecuteSwapWithFees(input: {
  isBuy: boolean
  usdcAmount: bigint
  tokenAmount: bigint
  platformFeeUsdc: bigint
  gasMode: SolanaGasMode
  gasTopUpUsdc: bigint
  nativeAmount: bigint
  treasury: PublicKey
  gasRecipient: PublicKey
  jupiterData: Buffer
  gasJupiterData: Buffer
}) {
  const hasGasRoute = input.gasJupiterData.length > 0
  const valid =
    input.jupiterData.length > 0 &&
    input.usdcAmount > 0n &&
    input.tokenAmount > 0n &&
    ((input.gasMode === "none" &&
      input.gasTopUpUsdc === 0n &&
      input.nativeAmount === 0n &&
      !hasGasRoute) ||
      (input.gasMode === "credit_only" &&
        input.gasTopUpUsdc === 0n &&
        input.nativeAmount === 0n &&
        !hasGasRoute) ||
      (input.gasMode === "separate" &&
        input.gasTopUpUsdc > 0n &&
        input.nativeAmount > 0n &&
        input.gasJupiterData.length > 0) ||
      (input.gasMode === "native_output" &&
        input.isBuy &&
        input.gasTopUpUsdc > 0n &&
        input.nativeAmount > 0n &&
        !hasGasRoute))
  if (!valid) {
    throw new Error("Invalid gas encoding.")
  }

  const header = Buffer.alloc(1 + 1 + 8 + 8 + 8 + 1 + 8 + 8 + 32 + 32)
  let offset = 0
  header[offset++] = VARIANT.ExecuteSwapWithFees
  header[offset++] = input.isBuy ? 1 : 0
  header.writeBigUInt64LE(input.usdcAmount, offset)
  offset += 8
  header.writeBigUInt64LE(input.tokenAmount, offset)
  offset += 8
  header.writeBigUInt64LE(input.platformFeeUsdc, offset)
  offset += 8
  header[offset++] = GAS_MODE[input.gasMode]
  header.writeBigUInt64LE(input.gasTopUpUsdc, offset)
  offset += 8
  header.writeBigUInt64LE(input.nativeAmount, offset)
  offset += 8
  header.set(input.treasury.toBytes(), offset)
  offset += 32
  header.set(input.gasRecipient.toBytes(), offset)
  return appendVecU8(
    appendVecU8(header, input.jupiterData),
    input.gasJupiterData
  )
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

export function encodeExecuteRelayDeposit(input: {
  relayOrderId: Uint8Array
  fundingChainId: bigint
  amount: bigint
  minDestAmount: bigint
  lockedCostUsdc: bigint
  platformFeeUsdc: bigint
  nonce: bigint
  deadline: bigint
  relayIxData: Buffer
}) {
  const header = Buffer.alloc(1 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 8)
  header[0] = VARIANT.ExecuteRelayDeposit
  header.set(Buffer.from(input.relayOrderId), 1)
  header.writeBigUInt64LE(input.fundingChainId, 33)
  header.writeBigUInt64LE(input.amount, 41)
  header.writeBigUInt64LE(input.minDestAmount, 49)
  header.writeBigUInt64LE(input.lockedCostUsdc, 57)
  header.writeBigUInt64LE(input.platformFeeUsdc, 65)
  header.writeBigUInt64LE(input.nonce, 73)
  header.writeBigInt64LE(input.deadline, 81)
  return appendVecU8(header, input.relayIxData)
}

export function encodeCreditRelayAsset(input: {
  relayOrderId: Uint8Array
  fundingChainId: bigint
  creditQuantity: bigint
  costUsdc: bigint
  minCreditQty: bigint
  maxCreditQty: bigint
  nonce: bigint
  deadline: bigint
}) {
  const data = Buffer.alloc(1 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 8)
  data[0] = VARIANT.CreditRelayAsset
  data.set(Buffer.from(input.relayOrderId), 1)
  data.writeBigUInt64LE(input.fundingChainId, 33)
  data.writeBigUInt64LE(input.creditQuantity, 41)
  data.writeBigUInt64LE(input.costUsdc, 49)
  data.writeBigUInt64LE(input.minCreditQty, 57)
  data.writeBigUInt64LE(input.maxCreditQty, 65)
  data.writeBigUInt64LE(input.nonce, 73)
  data.writeBigInt64LE(input.deadline, 81)
  return data
}

export function encodeExecuteRemoteRelaySell(input: {
  relayOrderId: Uint8Array
  fundingChainId: bigint
  sellQuantity: bigint
  minReturnUsdc: bigint
  nonce: bigint
  deadline: bigint
  relayIxData: Buffer
}) {
  const header = Buffer.alloc(1 + 32 + 8 + 8 + 8 + 8 + 8)
  header[0] = VARIANT.ExecuteRemoteRelaySell
  header.set(Buffer.from(input.relayOrderId), 1)
  header.writeBigUInt64LE(input.fundingChainId, 33)
  header.writeBigUInt64LE(input.sellQuantity, 41)
  header.writeBigUInt64LE(input.minReturnUsdc, 49)
  header.writeBigUInt64LE(input.nonce, 57)
  header.writeBigInt64LE(input.deadline, 65)
  return appendVecU8(header, input.relayIxData)
}

export function encodeExecuteRelayGasTopUp(input: {
  relayOrderId: Uint8Array
  overheadUsdc: bigint
  gasRecipient: PublicKey
  nonce: bigint
  deadline: bigint
  relayIxData: Buffer
}) {
  const header = Buffer.alloc(1 + 32 + 8 + 32 + 8 + 8)
  header[0] = VARIANT.ExecuteRelayGasTopUp
  header.set(Buffer.from(input.relayOrderId), 1)
  header.writeBigUInt64LE(input.overheadUsdc, 33)
  header.set(input.gasRecipient.toBytes(), 41)
  header.writeBigUInt64LE(input.nonce, 73)
  header.writeBigInt64LE(input.deadline, 81)
  return appendVecU8(header, input.relayIxData)
}

export function encodeReleaseRelayGasTopUp(input: {
  relayOrderId: Uint8Array
  refundUsdc: bigint
  nonce: bigint
  deadline: bigint
}) {
  const data = Buffer.alloc(1 + 32 + 8 + 8 + 8)
  data[0] = VARIANT.ReleaseRelayGasTopUp
  data.set(Buffer.from(input.relayOrderId), 1)
  data.writeBigUInt64LE(input.refundUsdc, 33)
  data.writeBigUInt64LE(input.nonce, 41)
  data.writeBigInt64LE(input.deadline, 49)
  return data
}

export function encodeCreditUsdcReturn(input: {
  relayOrderId: Uint8Array
  fundingChainId: bigint
  grossReturnUsdc: bigint
  quantityReleased: bigint
  costReleasedUsdc: bigint
  platformFeeUsdc: bigint
  nonce: bigint
  deadline: bigint
}) {
  const data = Buffer.alloc(1 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 8)
  data[0] = VARIANT.CreditUsdcReturn
  data.set(Buffer.from(input.relayOrderId), 1)
  data.writeBigUInt64LE(input.fundingChainId, 33)
  data.writeBigUInt64LE(input.grossReturnUsdc, 41)
  data.writeBigUInt64LE(input.quantityReleased, 49)
  data.writeBigUInt64LE(input.costReleasedUsdc, 57)
  data.writeBigUInt64LE(input.platformFeeUsdc, 65)
  data.writeBigUInt64LE(input.nonce, 73)
  data.writeBigInt64LE(input.deadline, 81)
  return data
}

export function encodeReleaseRelayDeposit(input: {
  relayOrderId: Uint8Array
  refundAmountUsdc: bigint
  lockedCostUsdc: bigint
  nonce: bigint
  deadline: bigint
}) {
  const data = Buffer.alloc(1 + 32 + 8 + 8 + 8 + 8)
  data[0] = VARIANT.ReleaseRelayDeposit
  data.set(Buffer.from(input.relayOrderId), 1)
  data.writeBigUInt64LE(input.refundAmountUsdc, 33)
  data.writeBigUInt64LE(input.lockedCostUsdc, 41)
  data.writeBigUInt64LE(input.nonce, 49)
  data.writeBigInt64LE(input.deadline, 57)
  return data
}

export function encodeRestoreRemoteRelayAsset(input: {
  relayOrderId: Uint8Array
  fundingChainId: bigint
  restoreQuantity: bigint
  restoreCostUsdc: bigint
  nonce: bigint
  deadline: bigint
}) {
  const data = Buffer.alloc(1 + 32 + 8 + 8 + 8 + 8 + 8)
  data[0] = VARIANT.RestoreRemoteRelayAsset
  data.set(Buffer.from(input.relayOrderId), 1)
  data.writeBigUInt64LE(input.fundingChainId, 33)
  data.writeBigUInt64LE(input.restoreQuantity, 41)
  data.writeBigUInt64LE(input.restoreCostUsdc, 49)
  data.writeBigUInt64LE(input.nonce, 57)
  data.writeBigInt64LE(input.deadline, 65)
  return data
}

export type { Hex }
