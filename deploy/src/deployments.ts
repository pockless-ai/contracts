import { readFile, rename, writeFile } from "node:fs/promises"
import type { DeploymentManifest } from "./manifest"
import type { Target } from "./config"

type Json = {
  evm: Record<string, Record<string, unknown>>
  solana: Record<string, Record<string, unknown>>
}

const publicRpc: Record<string, string> = {
  "1": "https://eth.llamarpc.com",
  "10": "https://mainnet.optimism.io",
  "56": "https://bsc-dataseed.bnbchain.org",
  "137": "https://polygon-rpc.com",
  "8453": "https://mainnet.base.org",
  "84532": "https://sepolia.base.org",
  "42161": "https://arb1.arbitrum.io/rpc",
}

export async function mergeDeployments(
  path: string,
  manifest: DeploymentManifest,
  targets: Target[]
) {
  const current = JSON.parse(await readFile(path, "utf8")) as Json
  for (const target of targets) {
    const state = manifest.targets[target.key]
    if (!state || state.status !== "complete") continue
    if (target.family === "evm" && state.address) {
      current.evm[target.key] = {
        ...(current.evm[target.key] ?? {}),
        name: target.name,
        tier: manifest.environment,
        chainId: target.chainId,
        rpc: publicRpc[target.key],
        usdc: target.usdc,
        implementation: state.address,
        status: "deployed",
        txHash: state.txHash,
        codeHash: state.codeHash,
        verifiedAt: state.verifiedAt,
        releaseCommit: manifest.releaseCommit,
      }
    }
    if (target.family === "solana" && state.programId) {
      current.solana[target.key] = {
        ...(current.solana[target.key] ?? {}),
        name: target.name,
        tier: manifest.environment,
        rpc:
          target.cluster === "devnet"
            ? "https://api.devnet.solana.com"
            : "https://api.mainnet-beta.solana.com",
        programId: state.programId,
        status: state.immutableAt ? "immutable" : "deployed",
        usdcMint: target.usdcMint,
        programHash: state.programHash,
        verifiedAt: state.verifiedAt,
        immutableAt: state.immutableAt,
        releaseCommit: manifest.releaseCommit,
      }
    }
  }
  const temporary = `${path}.${process.pid}.tmp`
  await writeFile(temporary, `${JSON.stringify(current, null, 2)}\n`)
  await rename(temporary, path)
}
