import { readFile } from "node:fs/promises"
import { basename, dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import { parseEnv } from "node:util"
import type { Environment } from "./config"

const deployRoot = join(dirname(fileURLToPath(import.meta.url)), "..")
const envKeys = [
  "BASE_SEPOLIA_RPC_URL",
  "ETHEREUM_RPC_URL",
  "BASE_RPC_URL",
  "ARBITRUM_RPC_URL",
  "OPTIMISM_RPC_URL",
  "POLYGON_RPC_URL",
  "BNB_RPC_URL",
  "SOLANA_DEVNET_RPC_URL",
  "SOLANA_MAINNET_RPC_URL",
  "BASE_SEPOLIA_USDC_ADDRESS",
  "EVM_FOUNDRY_ACCOUNT",
  "EVM_FOUNDRY_PASSWORD",
  "EVM_DEPLOYER_ADDRESS",
  "ETHERSCAN_API_KEY",
  "SOLANA_FEE_PAYER_KEYPAIR",
  "SOLANA_PROGRAM_KEYPAIR",
  "SOLANA_VERIFY_REPOSITORY_URL",
  "SOLANA_VERIFY_STATUS_URL",
  "SOLANA_VERIFY_TIMEOUT_SECONDS",
  "SOLANA_VERIFY_POLL_SECONDS",
  "SOLANA_VERIFY_AUTHORITY_MIN_LAMPORTS",
] as const

async function loadOptionalEnv(path: string) {
  try {
    return {
      name: basename(path),
      values: parseEnv(await readFile(path, "utf8")),
    }
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return undefined
    throw error
  }
}

export async function loadDeployEnv(
  environment: Environment,
  root = deployRoot
) {
  const [profile, shared] = await Promise.all([
    loadOptionalEnv(join(root, `.env.${environment}`)),
    loadOptionalEnv(join(root, ".env")),
  ])

  return {
    source: Object.fromEntries(
      envKeys.map((key) => [
        key,
        process.env[key] ?? profile?.values[key] ?? shared?.values[key],
      ])
    ) as Record<string, string | undefined>,
    loadedFiles: [profile?.name, shared?.name].filter(
      (value): value is string => value !== undefined
    ),
  }
}

export function resolveSolanaKeypairs(
  source: Record<string, string | undefined>
) {
  const feePayer = source.SOLANA_FEE_PAYER_KEYPAIR?.trim()
  if (!feePayer) throw new Error("SOLANA_FEE_PAYER_KEYPAIR is required")
  const programKeypair = source.SOLANA_PROGRAM_KEYPAIR?.trim()
  if (!programKeypair) throw new Error("SOLANA_PROGRAM_KEYPAIR is required")
  if (programKeypair === feePayer) {
    throw new Error(
      "SOLANA_PROGRAM_KEYPAIR must be separate from SOLANA_FEE_PAYER_KEYPAIR"
    )
  }
  return {
    feePayer,
    authority: feePayer,
    programKeypair,
  }
}
