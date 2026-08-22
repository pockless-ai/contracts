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
  swapIntentTypes,
  swapIntentDomain,
  encodeSessionGrant,
  encodeSessionRevoke,
  encodeSessionSignedRevoke,
  encodeSessionRotate,
  encodeSessionSetLimit,
  encodeExecuteSwap,
  encodeExecuteSwapWithFees,
  strategyIdFromCuid,
  revokeIntentTypes,
  swapBundleIntentTypes,
  swapBundleIntentDomain,
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
  encodeWithdrawAsset,
  encodeCloseStrategy,
  solanaStrategyIdFromCuid,
} from "./solana/strategy-spend"

export type {
  RevokeIntentMessage,
  SwapIntentMessage,
  SwapBundleIntentMessage,
} from "./evm/session-spend-7702"
