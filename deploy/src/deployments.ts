import { readFile, rename, writeFile } from "node:fs/promises"
import type { DeploymentManifest } from "./manifest"
import type { Target } from "./config"

type Json = {
  evm: Record<string, Record<string, unknown>>
  solana: Record<string, Record<string, unknown>>
}

function releaseHistory(entry: Record<string, unknown> | undefined) {
  return Array.isArray(entry?.releases)
    ? [...(entry.releases as Record<string, unknown>[])]
    : []
}

function archiveEvmRelease(
  entry: Record<string, unknown> | undefined,
  implementation: string,
  supersededAt: string
) {
  const previous = entry?.implementation
  const releases = releaseHistory(entry)
  if (
    typeof previous !== "string" ||
    previous === implementation ||
    previous === "0x0000000000000000000000000000000000000000"
  ) {
    return releases
  }
  if (
    !releases.some(
      (release) =>
        release.implementation === previous &&
        release.releaseCommit === entry?.releaseCommit
    )
  ) {
    releases.push({
      implementation: previous,
      txHash: entry?.txHash,
      codeHash: entry?.codeHash,
      verifiedAt: entry?.verifiedAt,
      releaseCommit: entry?.releaseCommit,
      supersededAt,
    })
  }
  return releases
}

function archiveSolanaRelease(
  entry: Record<string, unknown> | undefined,
  programHash: string | undefined,
  supersededAt: string
) {
  const previousHash = entry?.programHash
  const releases = releaseHistory(entry)
  if (
    typeof previousHash !== "string" ||
    previousHash === programHash
  ) {
    return releases
  }
  if (
    !releases.some(
      (release) =>
        release.programHash === previousHash &&
        release.releaseCommit === entry?.releaseCommit
    )
  ) {
    releases.push({
      programId: entry?.programId,
      programHash: previousHash,
      verifiedAt: entry?.verifiedAt,
      immutableAt: entry?.immutableAt,
      releaseCommit: entry?.releaseCommit,
      supersededAt,
    })
  }
  return releases
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
      const existing = current.evm[target.key]
      current.evm[target.key] = {
        ...(existing ?? {}),
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
        releases: archiveEvmRelease(
          existing,
          state.address,
          state.verifiedAt ?? manifest.updatedAt
        ),
      }
    }
    if (target.family === "solana" && state.programId) {
      const existing = current.solana[target.key]
      current.solana[target.key] = {
        ...(existing ?? {}),
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
        releases: archiveSolanaRelease(
          existing,
          state.programHash,
          state.verifiedAt ?? manifest.updatedAt
        ),
      }
    }
  }
  const temporary = `${path}.${process.pid}.tmp`
  await writeFile(temporary, `${JSON.stringify(current, null, 2)}\n`)
  await rename(temporary, path)
}
