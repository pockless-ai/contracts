import assert from "node:assert/strict"
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import test from "node:test"
import {
  evmDeployArgs,
  redactArgs,
  solanaDeployArgs,
  type RunCommand,
} from "../src/command"
import { resolveSolanaKeypairs } from "../src/env"
import { loadTargets } from "../src/config"
import { runDeploy } from "../src/deploy"
import { mergeDeployments } from "../src/deployments"
import {
  assertResumeCompatible,
  loadManifest,
  newManifest,
  saveManifest,
} from "../src/manifest"
import {
  assertInteractiveMainnet,
  immutablePhrase,
  mainnetPhrase,
} from "../src/safety"

const testUsdc = "0x1111111111111111111111111111111111111111"

test("network profiles enforce canonical mainnet targets and explicit testnet USDC", () => {
  assert.throws(() => loadTargets("testnet", {}), /BASE_SEPOLIA_USDC_ADDRESS/)
  const testnet = loadTargets("testnet", {
    BASE_SEPOLIA_USDC_ADDRESS: testUsdc,
  })
  assert.deepEqual(
    testnet.map((target) => target.name),
    ["base-sepolia", "devnet"]
  )
  const mainnet = loadTargets("mainnet", {})
  assert.deepEqual(
    mainnet
      .filter((target) => target.family === "evm")
      .map((target) => target.chainId),
    [1, 8453, 42161, 10, 137, 56]
  )
  assert.equal(
    mainnet.find((target) => target.family === "solana")?.usdcMint,
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
  )
})

test("manifest writes atomically, resumes, and rejects release mismatches", async () => {
  const directory = await mkdtemp(join(tmpdir(), "pockless-manifest-"))
  try {
    const path = join(directory, "testnet.json")
    const manifest = newManifest("testnet", "abc")
    manifest.targets["84532"] = {
      family: "evm",
      name: "base-sepolia",
      status: "complete",
      address: testUsdc,
    }
    await saveManifest(path, manifest)
    const loaded = await loadManifest(path)
    assert.equal(loaded?.targets["84532"]?.address, testUsdc)
    assert.doesNotThrow(() => assertResumeCompatible(loaded!, "testnet", "abc"))
    assert.throws(
      () => assertResumeCompatible(loaded!, "testnet", "def"),
      /does not match/
    )
    assert.equal((await readFile(path, "utf8")).endsWith("\n"), true)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})

test("Solana fee payer is the authority and program ID remains separate", () => {
  assert.deepEqual(
    resolveSolanaKeypairs({
      SOLANA_FEE_PAYER_KEYPAIR: "/secure/fee-payer.json",
      SOLANA_PROGRAM_KEYPAIR: "/secure/program.json",
    }),
    {
      feePayer: "/secure/fee-payer.json",
      authority: "/secure/fee-payer.json",
      programKeypair: "/secure/program.json",
    }
  )
  assert.throws(
    () =>
      resolveSolanaKeypairs({
        SOLANA_FEE_PAYER_KEYPAIR: "/secure/fee-payer.json",
      }),
    /SOLANA_PROGRAM_KEYPAIR/
  )
  assert.throws(
    () =>
      resolveSolanaKeypairs({
        SOLANA_FEE_PAYER_KEYPAIR: "/secure/fee-payer.json",
        SOLANA_PROGRAM_KEYPAIR: "/secure/fee-payer.json",
      }),
    /must be separate/
  )
  assert.throws(() => resolveSolanaKeypairs({}), /SOLANA_FEE_PAYER_KEYPAIR/)
})

test("command construction never uses raw keys and redacts signer paths", () => {
  const evm = evmDeployArgs({
    rpc: "https://user:secret@example.test",
    account: "release",
    sender: testUsdc,
    usdc: testUsdc,
  })
  assert.equal(evm.includes("--private-key"), false)
  assert.equal(evm.includes("--sender"), true)
  assert.equal(evm.includes("--from"), false)
  const redactedEvm = redactArgs(evm)
  assert.equal(redactedEvm[redactedEvm.indexOf("--rpc-url") + 1], "<redacted>")
  assert.equal(redactedEvm[redactedEvm.indexOf("--account") + 1], "<redacted>")
  const solana = solanaDeployArgs({
    artifact: "program.so",
    rpc: "https://rpc.example",
    feePayer: "/secret/payer.json",
    authority: "/secret/authority.json",
    programKeypair: "/secret/program.json",
  })
  const redacted = redactArgs(solana).join(" ")
  assert.equal(redacted.includes("/secret/"), false)
})

test("deployment merge preserves environments and adds release metadata", async () => {
  const directory = await mkdtemp(join(tmpdir(), "pockless-deployments-"))
  try {
    const path = join(directory, "deployments.json")
    await writeFile(
      path,
      '{"evm":{"1":{"name":"ethereum"}},"solana":{"mainnet-beta":{"name":"mainnet-beta"}}}'
    )
    const manifest = newManifest("testnet", "abc")
    manifest.targets["84532"] = {
      family: "evm",
      name: "base-sepolia",
      status: "complete",
      address: testUsdc,
      txHash: `0x${"1".repeat(64)}`,
      codeHash: `0x${"2".repeat(64)}`,
    }
    await mergeDeployments(
      path,
      manifest,
      loadTargets("testnet", { BASE_SEPOLIA_USDC_ADDRESS: testUsdc })
    )
    const merged = JSON.parse(await readFile(path, "utf8"))
    assert.equal(merged.evm["1"].name, "ethereum")
    assert.equal(merged.evm["84532"].tier, "testnet")
    assert.equal(merged.evm["84532"].releaseCommit, "abc")
    assert.equal(merged.evm["84532"].status, "deployed")
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})

test("mainnet and immutable confirmations are exact and require TTY", () => {
  assert.equal(mainnetPhrase("mainnet"), "DEPLOY POCKLESS MAINNET")
  assert.equal(
    immutablePhrase("mainnet-beta", "Program111"),
    "MAKE SOLANA mainnet-beta Program111 PERMANENTLY IMMUTABLE"
  )
  assert.throws(
    () =>
      assertInteractiveMainnet(
        "mainnet",
        { isTTY: false } as unknown as NodeJS.ReadStream,
        { isTTY: true } as unknown as NodeJS.WriteStream
      ),
    /interactive TTY/
  )
})

test("dry-run completes injected preflight without invoking command or RPC boundaries", async () => {
  const directory = await mkdtemp(join(tmpdir(), "pockless-dry-run-"))
  const manifestPath = join(directory, "testnet.json")
  let preflights = 0
  const forbiddenRun: RunCommand = async () => {
    throw new Error("broadcast boundary must not run")
  }
  try {
    const manifest = await runDeploy(
      {
        environment: "testnet",
        dryRun: true,
        skipTests: false,
        skipSolanaVerification: true,
        redeploy: false,
        safetyBufferPercent: 20,
        source: { BASE_SEPOLIA_USDC_ADDRESS: testUsdc },
      },
      {
        run: forbiddenRun,
        log: () => undefined,
        setup: async () => ({ releaseCommit: "dry-run-commit" }),
        preflight: async (target) => {
          preflights += 1
          return { artifactHash: `hash-${target.key}` }
        },
        manifestPath,
      }
    )
    assert.equal(preflights, 2)
    assert.equal(manifest.targets["84532"].status, "pending")
    assert.equal(manifest.targets.devnet.status, "pending")
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})

test("dry-run preserves a completed resumable deployment", async () => {
  const directory = await mkdtemp(join(tmpdir(), "pockless-dry-run-resume-"))
  const manifestPath = join(directory, "testnet.json")
  const completed = newManifest("testnet", "dry-run-commit")
  completed.targets["84532"] = {
    family: "evm",
    name: "base-sepolia",
    status: "complete",
    artifactHash: "hash-84532",
    address: testUsdc,
    txHash: `0x${"1".repeat(64)}`,
    codeHash: `0x${"2".repeat(64)}`,
  }
  await saveManifest(manifestPath, completed)
  try {
    const manifest = await runDeploy(
      {
        environment: "testnet",
        dryRun: true,
        skipTests: false,
        skipSolanaVerification: true,
        redeploy: false,
        safetyBufferPercent: 20,
        source: { BASE_SEPOLIA_USDC_ADDRESS: testUsdc },
      },
      {
        run: async () => {
          throw new Error("command boundary must not run")
        },
        log: () => undefined,
        setup: async () => ({ releaseCommit: "dry-run-commit" }),
        preflight: async (target) => ({ artifactHash: `hash-${target.key}` }),
        manifestPath,
      }
    )
    assert.equal(manifest.targets["84532"].status, "complete")
    assert.equal(manifest.targets.devnet.status, "pending")
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})
