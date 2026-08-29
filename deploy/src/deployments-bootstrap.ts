import { createHash } from "node:crypto"
import { readFile } from "node:fs/promises"
import { getAddress, isAddress, keccak256, type Hex } from "viem"
import type { Environment, Target } from "./config"
import type { DeploymentManifest, TargetState } from "./manifest"

export type DeploymentsRecord = {
  evm: Record<string, Record<string, unknown>>
  solana: Record<string, Record<string, unknown>>
}

export type CurrentArtifact = {
  artifactHash: string
  releaseHash: string
}

export type CommitRelation = "ahead" | "same" | "behind" | "unknown"

function isHex(value: string) {
  return /^0x[0-9a-fA-F]+$/.test(value)
}

export function solanaExecutableHash(bytes: Uint8Array) {
  let end = bytes.length
  while (end > 0 && bytes[end - 1] === 0) end -= 1
  return createHash("sha256").update(bytes.subarray(0, end)).digest("hex")
}

export async function loadDeploymentsRecord(path: string) {
  return JSON.parse(await readFile(path, "utf8")) as DeploymentsRecord
}

export function deploymentEntryForTarget(
  deployments: DeploymentsRecord,
  target: Target,
  environment: Environment
) {
  const entry =
    target.family === "evm"
      ? deployments.evm[target.key]
      : deployments.solana[target.key]
  if (!entry || entry.tier !== environment) {
    return undefined
  }
  if (entry.status !== "deployed" && entry.status !== "immutable") {
    return undefined
  }
  return entry
}

function normalizedHex(value: unknown) {
  return typeof value === "string" && isHex(value)
    ? value.toLowerCase()
    : undefined
}

function normalizedString(value: unknown) {
  return typeof value === "string" && value.trim() ? value.trim() : undefined
}

export function deploymentReleaseCommit(entry: Record<string, unknown>) {
  return normalizedString(entry.releaseCommit)
}

export function shouldBootstrapTarget(
  manifestTarget: TargetState | undefined,
  deploymentEntry: Record<string, unknown>,
  manifestReleaseCommit: string,
  currentCommit: string,
  deploymentRelation: CommitRelation
) {
  if (!manifestTarget) {
    return true
  }

  if (manifestTarget.status !== "complete") {
    return (
      manifestReleaseCommit !== currentCommit &&
      (deploymentRelation === "ahead" || deploymentRelation === "same")
    )
  }

  if (deploymentRelation === "ahead") {
    return true
  }
  if (deploymentRelation !== "same") {
    return false
  }

  if (manifestTarget.family === "evm") {
    const implementation = normalizedString(deploymentEntry.implementation)
    if (
      implementation &&
      isAddress(implementation) &&
      manifestTarget.address &&
      getAddress(implementation) !== getAddress(manifestTarget.address)
    ) {
      return true
    }
    const codeHash = normalizedHex(deploymentEntry.codeHash)
    if (
      codeHash &&
      manifestTarget.codeHash &&
      codeHash !== manifestTarget.codeHash.toLowerCase()
    ) {
      return true
    }
    return false
  }

  const programId = normalizedString(deploymentEntry.programId)
  if (programId && manifestTarget.programId && programId !== manifestTarget.programId) {
    return true
  }
  const programHash = normalizedString(deploymentEntry.programHash)
  if (
    programHash &&
    manifestTarget.programHash &&
    programHash !== manifestTarget.programHash
  ) {
    return true
  }
  return false
}

export function targetStateFromDeployment(
  target: Target,
  entry: Record<string, unknown>,
  current: CurrentArtifact,
  currentCommit: string
): TargetState {
  const verifiedAt = normalizedString(entry.verifiedAt)
  const unchangedRelease =
    normalizedString(entry.releaseCommit) === currentCommit
  const recordedProgramHash = normalizedString(entry.programHash)
  const canonicalProgramHash =
    target.family === "solana" &&
    recordedProgramHash === current.artifactHash
      ? current.releaseHash
      : recordedProgramHash
  const unchangedArtifact =
    target.family === "evm"
      ? normalizedHex(entry.codeHash) === current.releaseHash.toLowerCase()
      : canonicalProgramHash === current.releaseHash

  const base: TargetState = {
    family: target.family,
    name: target.name,
    status: "complete",
    verificationStatus: "verified",
    verifiedAt,
    ...((target.family === "evm" ? unchangedRelease : true) &&
    unchangedArtifact
      ? { artifactHash: current.artifactHash }
      : {}),
  }

  if (target.family === "evm") {
    const implementation = normalizedString(entry.implementation)
    return {
      ...base,
      address: implementation && isAddress(implementation) ? implementation : undefined,
      txHash: normalizedString(entry.txHash),
      codeHash: normalizedHex(entry.codeHash),
    }
  }

  return {
    ...base,
    programId: normalizedString(entry.programId),
    programHash: canonicalProgramHash,
    immutableAt: normalizedString(entry.immutableAt),
  }
}

export function bootstrapManifestFromDeployments(input: {
  manifest: DeploymentManifest
  deployments: DeploymentsRecord
  targets: Target[]
  environment: Environment
  currentCommit: string
  currentArtifacts: Map<string, CurrentArtifact>
  deploymentRelations: Map<string, CommitRelation>
  log: (message: string) => void
}) {
  let changed = false
  for (const target of input.targets) {
    const entry = deploymentEntryForTarget(
      input.deployments,
      target,
      input.environment
    )
    if (!entry) continue

    const current = input.currentArtifacts.get(target.key)
    if (!current) continue

    const existing = input.manifest.targets[target.key]
    if (
      target.family === "solana" &&
      existing?.family === "solana" &&
      existing.programHash === current.artifactHash
    ) {
      existing.programHash = current.releaseHash
      existing.artifactHash = current.artifactHash
      input.log(
        `${target.name}: normalized legacy Solana program hash in manifest`
      )
      changed = true
    }

    if (
      !shouldBootstrapTarget(
        existing,
        entry,
        input.manifest.releaseCommit,
        input.currentCommit,
        input.deploymentRelations.get(target.key) ?? "unknown"
      )
    ) {
      continue
    }

    input.manifest.targets[target.key] = targetStateFromDeployment(
      target,
      entry,
      current,
      input.currentCommit
    )
    input.log(
      `${target.name}: bootstrapped manifest from deployments.json${
        existing ? " (local manifest stale)" : ""
      }`
    )
    changed = true
  }
  return changed
}

export async function loadEvmCurrentArtifact(artifactPath: string) {
  const raw = JSON.parse(await readFile(artifactPath, "utf8")) as {
    deployedBytecode?: { object?: Hex }
  }
  const object = raw.deployedBytecode?.object
  if (!object) {
    throw new Error("Foundry artifact is missing deployed bytecode")
  }
  const artifactHash = createHash("sha256")
    .update(await readFile(artifactPath))
    .digest("hex")
  return {
    artifactHash,
    releaseHash: keccak256(object),
  }
}

export async function loadSolanaCurrentArtifact(artifactPath: string) {
  const bytes = await readFile(artifactPath)
  return {
    artifactHash: createHash("sha256").update(bytes).digest("hex"),
    releaseHash: solanaExecutableHash(bytes),
  }
}

export async function loadCurrentArtifacts(
  targets: Target[],
  input: { evmArtifactPath: string; solanaArtifactPath: string }
) {
  const [evm, solana] = await Promise.all([
    loadEvmCurrentArtifact(input.evmArtifactPath),
    loadSolanaCurrentArtifact(input.solanaArtifactPath),
  ])
  const map = new Map<string, CurrentArtifact>()
  for (const target of targets) {
    map.set(target.key, target.family === "evm" ? evm : solana)
  }
  return map
}
