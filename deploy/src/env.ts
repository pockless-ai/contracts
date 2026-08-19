import { loadEnvFile } from "node:process"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

try {
  loadEnvFile(join(dirname(fileURLToPath(import.meta.url)), "../.env"))
} catch (error) {
  if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error
}

export const env: Record<string, string | undefined> = {
  BASE_SEPOLIA_RPC_URL: process.env.BASE_SEPOLIA_RPC_URL,
  ETHEREUM_RPC_URL: process.env.ETHEREUM_RPC_URL,
  BASE_RPC_URL: process.env.BASE_RPC_URL,
  ARBITRUM_RPC_URL: process.env.ARBITRUM_RPC_URL,
  OPTIMISM_RPC_URL: process.env.OPTIMISM_RPC_URL,
  POLYGON_RPC_URL: process.env.POLYGON_RPC_URL,
  BNB_RPC_URL: process.env.BNB_RPC_URL,
  SOLANA_DEVNET_RPC_URL: process.env.SOLANA_DEVNET_RPC_URL,
  SOLANA_MAINNET_RPC_URL: process.env.SOLANA_MAINNET_RPC_URL,
  BASE_SEPOLIA_USDC_ADDRESS: process.env.BASE_SEPOLIA_USDC_ADDRESS,
  EVM_FOUNDRY_ACCOUNT: process.env.EVM_FOUNDRY_ACCOUNT,
  EVM_DEPLOYER_ADDRESS: process.env.EVM_DEPLOYER_ADDRESS,
  ETHERSCAN_API_KEY: process.env.ETHERSCAN_API_KEY,
  SOLANA_FEE_PAYER_KEYPAIR: process.env.SOLANA_FEE_PAYER_KEYPAIR,
  SOLANA_UPGRADE_AUTHORITY_KEYPAIR:
    process.env.SOLANA_UPGRADE_AUTHORITY_KEYPAIR,
  SOLANA_PROGRAM_KEYPAIR: process.env.SOLANA_PROGRAM_KEYPAIR,
  SOLANA_VERIFY_REPOSITORY_URL: process.env.SOLANA_VERIFY_REPOSITORY_URL,
  SOLANA_VERIFY_STATUS_URL: process.env.SOLANA_VERIFY_STATUS_URL,
  SOLANA_VERIFY_TIMEOUT_SECONDS: process.env.SOLANA_VERIFY_TIMEOUT_SECONDS,
  SOLANA_VERIFY_POLL_SECONDS: process.env.SOLANA_VERIFY_POLL_SECONDS,
  SOLANA_VERIFY_AUTHORITY_MIN_LAMPORTS:
    process.env.SOLANA_VERIFY_AUTHORITY_MIN_LAMPORTS,
}
