import { mkdir, readFile, rename, writeFile } from "node:fs/promises"
import { dirname } from "node:path"
import type { Environment } from "./config"

export type StepStatus = "pending" | "running" | "complete" | "failed"

export type TargetState = {
  family: "evm" | "solana"
  name: string
  status: StepStatus
  error?: string
  artifactHash?: string
  address?: string
  txHash?: string
  codeHash?: string
  programId?: string
  programHash?: string
  deploySignature?: string
  verificationStatus?: "verified" | "pending" | "failed"
  verifiedAt?: string
  immutableAt?: string
  immutableSignature?: string
  immutableStatus?: "immutable"
}

export type DeploymentManifest = {
  version: 1
  environment: Environment
  releaseCommit: string
  updatedAt: string
  targets: Record<string, TargetState>
}

export function newManifest(
  environment: Environment,
  releaseCommit: string
): DeploymentManifest {
  return {
    version: 1,
    environment,
    releaseCommit,
    updatedAt: new Date().toISOString(),
    targets: {},
  }
}

export async function loadManifest(path: string) {
  try {
    return JSON.parse(await readFile(path, "utf8")) as DeploymentManifest
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return undefined
    throw error
  }
}

export async function saveManifest(path: string, manifest: DeploymentManifest) {
  manifest.updatedAt = new Date().toISOString()
  await mkdir(dirname(path), { recursive: true, mode: 0o700 })
  const temporary = `${path}.${process.pid}.tmp`
  await writeFile(temporary, `${JSON.stringify(manifest, null, 2)}\n`, {
    mode: 0o600,
  })
  await rename(temporary, path)
}

export function assertResumeCompatible(
  manifest: DeploymentManifest,
  environment: Environment,
  releaseCommit: string
) {
  if (manifest.environment !== environment) {
    throw new Error("manifest environment does not match requested environment")
  }
  if (manifest.releaseCommit !== releaseCommit) {
    throw new Error(
      `manifest release commit ${manifest.releaseCommit} does not match ${releaseCommit}`
    )
  }
}
