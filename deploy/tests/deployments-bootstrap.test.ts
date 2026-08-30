import assert from "node:assert/strict"
import test from "node:test"
import {
  bootstrapManifestFromDeployments,
  deploymentEntryForTarget,
  shouldBootstrapTarget,
  targetStateFromDeployment,
} from "../src/deployments-bootstrap"
import { loadTargets } from "../src/config"
import { newManifest } from "../src/manifest"

const testUsdc = "0x1111111111111111111111111111111111111111"

test("deploymentEntryForTarget ignores other environments", () => {
  const targets = loadTargets("testnet", {
    BASE_SEPOLIA_USDC_ADDRESS: testUsdc,
  })
  const entry = deploymentEntryForTarget(
    {
      evm: {
        "84532": {
          tier: "mainnet",
          status: "deployed",
          implementation: testUsdc,
        },
      },
      solana: {},
    },
    targets[0]!,
    "testnet"
  )
  assert.equal(entry, undefined)
})

test("shouldBootstrapTarget preserves resumable current-release targets", () => {
  const deployment = {
    releaseCommit: "deployed-commit",
    implementation: "0x2222222222222222222222222222222222222222",
    codeHash: `0x${"a".repeat(64)}`,
    txHash: `0x${"b".repeat(64)}`,
  }
  assert.equal(
    shouldBootstrapTarget(
      undefined,
      deployment,
      "manifest-commit",
      "current-commit",
      "ahead"
    ),
    true
  )
  assert.equal(
    shouldBootstrapTarget(
      {
        family: "evm",
        name: "base-sepolia",
        status: "failed",
        address: testUsdc,
      },
      deployment,
      "current-commit",
      "current-commit",
      "behind"
    ),
    false
  )
  assert.equal(
    shouldBootstrapTarget(
      {
        family: "evm",
        name: "base-sepolia",
        status: "running",
        address: testUsdc,
      },
      deployment,
      "current-commit",
      "current-commit",
      "ahead"
    ),
    false
  )
  assert.equal(
    shouldBootstrapTarget(
      {
        family: "evm",
        name: "base-sepolia",
        status: "failed",
        address: testUsdc,
      },
      deployment,
      "manifest-commit",
      "current-commit",
      "ahead"
    ),
    true
  )
  assert.equal(
    shouldBootstrapTarget(
      {
        family: "evm",
        name: "base-sepolia",
        status: "complete",
        address: deployment.implementation,
        codeHash: deployment.codeHash,
      },
      deployment,
      "deployed-commit",
      "current-commit",
      "same"
    ),
    false
  )
  assert.equal(
    shouldBootstrapTarget(
      {
        family: "evm",
        name: "base-sepolia",
        status: "complete",
        address: testUsdc,
        codeHash: deployment.codeHash,
      },
      deployment,
      "manifest-commit",
      "current-commit",
      "ahead"
    ),
    true
  )
})

test("bootstrapManifestFromDeployments fills missing targets and pins unchanged artifact hashes", () => {
  const targets = loadTargets("testnet", {
    BASE_SEPOLIA_USDC_ADDRESS: testUsdc,
  })
  const manifest = newManifest("testnet", "manifest-commit")
  manifest.targets["84532"] = {
    family: "evm",
    name: "base-sepolia",
    status: "failed",
    address: testUsdc,
    error: "devnet completed deployment no longer matches on-chain program",
  }
  const logs: string[] = []
  const changed = bootstrapManifestFromDeployments({
    manifest,
    deployments: {
      evm: {
        "84532": {
          tier: "testnet",
          status: "deployed",
          implementation: "0x349f9C63300Fc12ad5Cb8301c60e017401268389",
          txHash: `0x${"1".repeat(64)}`,
          codeHash: `0x${"2".repeat(64)}`,
          releaseCommit: "deployed-commit",
          verifiedAt: "2026-08-25T20:33:15.265Z",
        },
      },
      solana: {
        devnet: {
          tier: "testnet",
          status: "deployed",
          programId: "Program111111111111111111111111111111111111111",
          programHash: "solana-program-hash",
          releaseCommit: "current-commit",
          verifiedAt: "2026-08-25T20:43:28.651Z",
        },
      },
    },
    targets,
    environment: "testnet",
    currentCommit: "current-commit",
    currentArtifacts: new Map([
      [
        "84532",
        {
          artifactHash: "evm-artifact-hash",
          releaseHash: `0x${"3".repeat(64)}`,
        },
      ],
      [
        "devnet",
        {
          artifactHash: "solana-artifact-hash",
          releaseHash: "solana-program-hash",
        },
      ],
    ]),
    deploymentRelations: new Map([
      ["84532", "ahead" as const],
      ["devnet", "same" as const],
    ]),
    log: (message) => logs.push(message),
  })

  assert.equal(changed, true)
  assert.equal(
    manifest.targets["84532"]?.address,
    "0x349f9C63300Fc12ad5Cb8301c60e017401268389"
  )
  assert.equal(manifest.targets["84532"]?.artifactHash, undefined)
  assert.equal(manifest.targets.devnet?.artifactHash, "solana-artifact-hash")
  assert.equal(logs.length, 2)
})

test("targetStateFromDeployment keeps artifact hash when release and artifact are unchanged", () => {
  const targets = loadTargets("testnet", {
    BASE_SEPOLIA_USDC_ADDRESS: testUsdc,
  })
  const state = targetStateFromDeployment(
    targets[1]!,
    {
      tier: "testnet",
      status: "deployed",
      programId: "Program111111111111111111111111111111111111111",
      programHash: "same-hash",
      releaseCommit: "same-commit",
    },
    { artifactHash: "artifact-hash", releaseHash: "same-hash" }
  )
  assert.equal(state.artifactHash, "artifact-hash")
})

test("targetStateFromDeployment restores EVM artifact identity from deployments", () => {
  const targets = loadTargets("testnet", {
    BASE_SEPOLIA_USDC_ADDRESS: testUsdc,
  })
  const state = targetStateFromDeployment(
    targets[0]!,
    {
      tier: "testnet",
      status: "deployed",
      implementation: testUsdc,
      codeHash: `0x${"1".repeat(64)}`,
      artifactHash: "artifact-hash",
      releaseCommit: "previous-commit",
      verifiedAt: "2026-08-25T20:33:15.265Z",
    },
    {
      artifactHash: "artifact-hash",
      releaseHash: `0x${"2".repeat(64)}`,
    }
  )

  assert.equal(state.artifactHash, "artifact-hash")
  assert.equal(state.verificationStatus, "verified")
})

test("targetStateFromDeployment does not promote unverified deployments", () => {
  const targets = loadTargets("testnet", {
    BASE_SEPOLIA_USDC_ADDRESS: testUsdc,
  })
  const state = targetStateFromDeployment(
    targets[1]!,
    {
      tier: "testnet",
      status: "deployed",
      programId: "Program111111111111111111111111111111111111111",
      programHash: "same-hash",
      artifactHash: "artifact-hash",
      verificationStatus: "pending",
    },
    { artifactHash: "artifact-hash", releaseHash: "same-hash" }
  )

  assert.equal(state.verificationStatus, "pending")
  assert.equal(state.verifiedAt, undefined)
})

test("targetStateFromDeployment normalizes legacy full-file Solana hashes", () => {
  const targets = loadTargets("testnet", {
    BASE_SEPOLIA_USDC_ADDRESS: testUsdc,
  })
  const state = targetStateFromDeployment(
    targets[1]!,
    {
      tier: "testnet",
      status: "deployed",
      programId: "Program111111111111111111111111111111111111111",
      programHash: "legacy-full-file-hash",
      releaseCommit: "previous-commit",
    },
    {
      artifactHash: "legacy-full-file-hash",
      releaseHash: "canonical-executable-hash",
    }
  )

  assert.equal(state.programHash, "canonical-executable-hash")
  assert.equal(state.artifactHash, "legacy-full-file-hash")
})

test("bootstrap leaves failed Solana targets unchanged without stale bootstrap", () => {
  const targets = loadTargets("testnet", {
    BASE_SEPOLIA_USDC_ADDRESS: testUsdc,
  })
  const manifest = newManifest("testnet", "current-commit")
  manifest.targets.devnet = {
    family: "solana",
    name: "devnet",
    status: "failed",
    artifactHash: "legacy-full-file-hash",
    programId: "Program111111111111111111111111111111111111111",
    programHash: "legacy-full-file-hash",
    error: "completed deployment no longer matches on-chain program",
  }

  const changed = bootstrapManifestFromDeployments({
    manifest,
    deployments: {
      evm: {},
      solana: {
        devnet: {
          tier: "testnet",
          status: "deployed",
          programId: "Program111111111111111111111111111111111111111",
          programHash: "legacy-full-file-hash",
          releaseCommit: "previous-commit",
        },
      },
    },
    targets,
    environment: "testnet",
    currentCommit: "current-commit",
    currentArtifacts: new Map([
      [
        "devnet",
        {
          artifactHash: "legacy-full-file-hash",
          releaseHash: "canonical-executable-hash",
        },
      ],
    ]),
    deploymentRelations: new Map([["devnet", "behind"]]),
    log: () => undefined,
  })

  assert.equal(changed, false)
  assert.equal(manifest.targets.devnet?.status, "failed")
  assert.equal(manifest.targets.devnet?.programHash, "legacy-full-file-hash")
  assert.equal(manifest.targets.devnet?.artifactHash, "legacy-full-file-hash")
})
