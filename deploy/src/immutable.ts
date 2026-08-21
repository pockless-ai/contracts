import { createHash } from "node:crypto"
import { access, mkdtemp, readFile, rm, stat } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { checked, runCommand, type RunCommand } from "./command"
import { loadTargets, requiredRpc, type Environment } from "./config"
import { mergeDeployments } from "./deployments"
import { env } from "./env"
import { loadManifest, saveManifest } from "./manifest"
import { immutablePhrase, confirmExact } from "./safety"
import { dirname } from "node:path"
import { fileURLToPath } from "node:url"

const contractsRoot = join(dirname(fileURLToPath(import.meta.url)), "../..")

type ProgramInfo = {
  authority?: string | null
  upgradeAuthority?: string | null
}

async function onChainProgramHash(
  run: RunCommand,
  rpc: string,
  programId: string
) {
  const directory = await mkdtemp(join(tmpdir(), "pockless-program-"))
  const output = join(directory, "program.so")
  try {
    await checked(run, "solana", [
      "program",
      "dump",
      programId,
      output,
      "--url",
      rpc,
    ])
    return createHash("sha256")
      .update(await readFile(output))
      .digest("hex")
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}

export async function runImmutable(
  environment: Environment,
  source: Record<string, string | undefined> = env,
  run: RunCommand = runCommand
) {
  const target = loadTargets(environment, source).find(
    (item) => item.family === "solana"
  )
  if (!target || target.family !== "solana")
    throw new Error("Solana target not configured")
  const path = join(contractsRoot, `deploy/.deploy/${environment}.json`)
  const manifest = await loadManifest(path)
  if (!manifest)
    throw new Error(`no successful ${environment} deployment manifest found`)
  const state = manifest.targets[target.key]
  if (
    !state ||
    state.status !== "complete" ||
    state.verificationStatus !== "verified" ||
    !state.programId ||
    !state.programHash
  ) {
    throw new Error(
      "Solana deployment must be complete and publicly verified before immutability"
    )
  }
  if (state.immutableAt)
    throw new Error("Solana program is already recorded as immutable")
  const rpc = requiredRpc(target, source)
  const feePayerPath = source.SOLANA_FEE_PAYER_KEYPAIR?.trim()
  if (!feePayerPath) throw new Error("SOLANA_FEE_PAYER_KEYPAIR is required")
  const authorityPath = feePayerPath
  await access(feePayerPath)
  const mode = (await stat(feePayerPath)).mode & 0o777
  if ((mode & 0o077) !== 0) {
    throw new Error(
      `fee payer permissions are ${mode.toString(8)}; run chmod 600 on the temporary file`
    )
  }
  const authority = (
    await checked(run, "solana-keygen", ["pubkey", authorityPath])
  ).stdout.trim()
  const before = JSON.parse(
    (
      await checked(run, "solana", [
        "program",
        "show",
        state.programId,
        "--url",
        rpc,
        "--output",
        "json",
      ])
    ).stdout
  ) as ProgramInfo
  const currentAuthority = before.authority ?? before.upgradeAuthority
  if (currentAuthority !== authority) {
    throw new Error(
      `on-chain upgrade authority does not match the supplied authority keypair`
    )
  }
  const hash = await onChainProgramHash(run, rpc, state.programId)
  if (hash !== state.programHash) {
    throw new Error(
      "on-chain program hash does not match the recorded verified release"
    )
  }
  await confirmExact(immutablePhrase(target.cluster, state.programId))
  const result = await checked(run, "solana", [
    "program",
    "set-upgrade-authority",
    state.programId,
    "--final",
    "--upgrade-authority",
    authorityPath,
    "--keypair",
    feePayerPath,
    "--url",
    rpc,
  ])
  const after = JSON.parse(
    (
      await checked(run, "solana", [
        "program",
        "show",
        state.programId,
        "--url",
        rpc,
        "--output",
        "json",
      ])
    ).stdout
  ) as ProgramInfo
  if ((after.authority ?? after.upgradeAuthority) != null) {
    throw new Error(
      "immutability transaction completed but authority is not none"
    )
  }
  state.immutableAt = new Date().toISOString()
  state.immutableSignature = result.stdout.match(
    /[1-9A-HJ-NP-Za-km-z]{64,88}/
  )?.[0]
  state.immutableStatus = "immutable"
  await saveManifest(path, manifest)
  await mergeDeployments(
    join(contractsRoot, "docs/deployments.json"),
    manifest,
    [target]
  )
  return manifest
}
