import { join, dirname } from "node:path"
import { fileURLToPath } from "node:url"
import { getAddress, type Address } from "viem"
import { loadTargets, requiredRpc } from "../../../src/config"
import { env } from "../../../src/env"
import { loadManifest } from "../../../src/manifest"

const deployRoot = join(dirname(fileURLToPath(import.meta.url)), "../../..")
const contractsRoot = join(deployRoot, "..")

export type SmokeConfig = {
  evm: {
    rpc: string
    chainId: number
    usdc: Address
    implementation: Address
    owner: Address
    account: string
    foundryPassword?: string
  }
  solana: {
    rpc: string
    programId: string
    usdcMint: string
    ownerKeypairPath: string
  }
}

export function smokeEnabled() {
  return process.env.LIVE_SMOKE === "1"
}

export async function loadSmokeConfig(): Promise<SmokeConfig> {
  if (!smokeEnabled()) {
    throw new Error("Set LIVE_SMOKE=1 to run testnet smoke tests")
  }

  const owner = env.EVM_DEPLOYER_ADDRESS?.trim()
  const account = env.EVM_FOUNDRY_ACCOUNT?.trim()
  const ownerKeypairPath = env.SOLANA_FEE_PAYER_KEYPAIR?.trim()
  if (!owner || !account) {
    throw new Error("EVM_DEPLOYER_ADDRESS and EVM_FOUNDRY_ACCOUNT are required")
  }
  if (!ownerKeypairPath) {
    throw new Error("SOLANA_FEE_PAYER_KEYPAIR is required")
  }

  const manifest = await loadManifest(
    join(deployRoot, ".deploy/testnet.json")
  )
  if (!manifest) {
    throw new Error(
      "No testnet deployment manifest found at deploy/.deploy/testnet.json"
    )
  }

  const evmState = manifest.targets["84532"]
  const solanaState = manifest.targets.devnet
  if (
    !evmState ||
    evmState.status !== "complete" ||
    !evmState.address
  ) {
    throw new Error("Base Sepolia deployment is not complete in the manifest")
  }
  if (
    !solanaState ||
    solanaState.status !== "complete" ||
    !solanaState.programId
  ) {
    throw new Error("Solana devnet deployment is not complete in the manifest")
  }

  const targets = loadTargets("testnet", env)
  const evmTarget = targets.find((target) => target.family === "evm")
  const solanaTarget = targets.find((target) => target.family === "solana")
  if (!evmTarget || evmTarget.family !== "evm") {
    throw new Error("Base Sepolia target is not configured")
  }
  if (!solanaTarget || solanaTarget.family !== "solana") {
    throw new Error("Solana devnet target is not configured")
  }

  return {
    evm: {
      rpc: requiredRpc(evmTarget, env),
      chainId: evmTarget.chainId,
      usdc: evmTarget.usdc,
      implementation: getAddress(evmState.address),
      owner: getAddress(owner),
      account,
      foundryPassword: env.EVM_FOUNDRY_PASSWORD?.trim() || undefined,
    },
    solana: {
      rpc: requiredRpc(solanaTarget, env),
      programId: solanaState.programId,
      usdcMint: solanaTarget.usdcMint,
      ownerKeypairPath,
    },
  }
}

export { contractsRoot, deployRoot }

export function assertEvmSignerAccess(config: SmokeConfig["evm"]) {
  if (config.foundryPassword || process.stdin.isTTY) return
  throw new Error(
    "Base Sepolia smoke needs EVM_FOUNDRY_PASSWORD in deploy/.env when stdin is not a TTY"
  )
}
