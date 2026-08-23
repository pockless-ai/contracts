export const ZEROX_ALLOWANCE_HOLDER =
  "0x0000000000001fF3684f28c67538d4D072C22734" as const

export const JUPITER_V6_PROGRAM_ID =
  "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" as const

export const SOLANA_USDC_MINT =
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" as const

export {
  sessionSpend7702Abi,
  SESSION_SPEND_NAME,
  SESSION_SPEND_VERSION,
  SESSION_SPEND_V2_VERSION,
  EVM_NATIVE_TOKEN,
  ZEROX_NATIVE_TOKEN,
  GasFundingMode,
  swapIntentTypes,
  swapIntentDomain,
  encodeSessionGrant,
  encodeSessionRevoke,
  encodeSessionSignedRevoke,
  encodeSessionRotate,
  encodeSessionSetLimit,
  encodeExecuteSwap,
  encodeExecuteSwapWithFees,
  encodeExecuteSwapWithFeesV2,
  strategyIdFromCuid,
  revokeIntentTypes,
  swapBundleIntentTypes,
  swapBundleIntentDomain,
  swapBundleIntentV2Types,
  swapBundleIntentV2Domain,
} from "./evm/session-spend-7702"

export {
  WALLET_SEED,
  STRATEGY_SEED,
  VAULT_SEED,
  ASSET_SEED,
  AUTHORITY_SEED,
  strategyPda,
  vaultPda,
  assetPda,
  walletPda,
  authorityPda,
  encodeInitWallet,
  encodeInitStrategy,
  encodeSetLimit,
  encodeRotateSession,
  encodeRevoke,
  encodeExecuteSwap as encodeSolanaExecuteSwap,
  encodeExecuteSwapWithFees as encodeSolanaExecuteSwapWithFees,
  encodeExecuteSwapWithFeesV2 as encodeSolanaExecuteSwapWithFeesV2,
  encodeWithdrawAsset,
  encodeCloseStrategy,
  solanaStrategyIdFromCuid,
} from "./solana/strategy-spend"

export type { SolanaGasMode } from "./solana/strategy-spend"

export type {
  RevokeIntentMessage,
  SwapIntentMessage,
  SwapBundleIntentMessage,
  SwapBundleIntentV2Message,
} from "./evm/session-spend-7702"
