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
import { runDeploy, waitForRuntimeCode } from "../src/deploy"
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
  upgradePhrase,
} from "../src/safety"

const testUsdc = "0x1111111111111111111111111111111111111111"

test("runtime code polling retries empty code and succeeds", async () => {
  let calls = 0
  const code = await waitForRuntimeCode(
    {
      getCode: async () => {
        calls += 1
        return calls === 1 ? "0x" : "0x6000"
      },
    },
    testUsdc,
    () => undefined,
    "base-sepolia",
    { attempts: 2, delayMs: 0 }
  )

  assert.equal(code, "0x6000")
  assert.equal(calls, 2)
})

test("runtime code polling retries transient RPC rejection and succeeds", async () => {
  let calls = 0
  const logs: string[] = []
  const code = await waitForRuntimeCode(
    {
      getCode: async () => {
        calls += 1
        if (calls === 1) throw new Error("temporary gateway failure")
        return "0x6000"
      },
    },
    testUsdc,
    (message) => logs.push(message),
    "base-sepolia",
    { attempts: 2, delayMs: 0 }
  )

  assert.equal(code, "0x6000")
  assert.equal(calls, 2)
  assert.match(logs[0]!, /temporary gateway failure/)
})

test("runtime code polling exhaustion reports the last RPC error", async () => {
  let calls = 0
  await assert.rejects(
    waitForRuntimeCode(
      {
        getCode: async () => {
          calls += 1
          if (calls === 2) throw new Error("RPC rate limit")
          return "0x"
        },
      },
      testUsdc,
      () => undefined,
      "base-sepolia",
      { attempts: 3, delayMs: 0 }
    ),
    /base-sepolia deployment has no runtime code; last RPC error: RPC rate limit/
  )
  assert.equal(calls, 3)
})

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
  assert.equal(evm.includes("--sender"), false)
  assert.equal(evm.includes("--from"), true)
  assert.deepEqual(evm.slice(-4), [
    "--broadcast",
    "--json",
    "--constructor-args",
    testUsdc,
  ])
  const redactedEvm = redactArgs(evm)
  assert.equal(redactedEvm[redactedEvm.indexOf("--rpc-url") + 1], "<redacted>")
  assert.equal(redactedEvm[redactedEvm.indexOf("--account") + 1], "<redacted>")
  assert.equal(redactedEvm[redactedEvm.indexOf("--from") + 1], "<redacted>")
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

    const upgraded = newManifest("testnet", "def")
    upgraded.targets["84532"] = {
      family: "evm",
      name: "base-sepolia",
      status: "complete",
      address: "0x2222222222222222222222222222222222222222",
      txHash: `0x${"3".repeat(64)}`,
      codeHash: `0x${"4".repeat(64)}`,
      verifiedAt: "2026-08-21T00:00:00.000Z",
    }
    await mergeDeployments(
      path,
      upgraded,
      loadTargets("testnet", { BASE_SEPOLIA_USDC_ADDRESS: testUsdc })
    )
    const afterUpgrade = JSON.parse(await readFile(path, "utf8"))
    assert.equal(afterUpgrade.evm["84532"].releaseCommit, "def")
    assert.equal(afterUpgrade.evm["84532"].releases.length, 1)
    assert.equal(afterUpgrade.evm["84532"].releases[0].implementation, testUsdc)
    assert.equal(afterUpgrade.evm["84532"].releases[0].releaseCommit, "abc")
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})

test("mainnet and immutable confirmations are exact and require TTY", () => {
  assert.equal(mainnetPhrase("mainnet"), "DEPLOY POCKLESS MAINNET")
  assert.equal(upgradePhrase("mainnet"), "UPGRADE POCKLESS MAINNET")
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
        operation: "deploy",
        forceBroadcast: false,
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

test(
  "mainnet dry-run checks targets concurrently and prints one funding summary",
  { timeout: 2_000 },
  async () => {
    const directory = await mkdtemp(join(tmpdir(), "pockless-funding-summary-"))
    const manifestPath = join(directory, "mainnet.json")
    const logs: string[] = []
    let started = 0
    const attempts = new Map<string, number>()
    let release: () => void = () => undefined
    const allStarted = new Promise<void>((resolve) => {
      release = resolve
    })
    try {
      await runDeploy(
        {
          environment: "mainnet",
          dryRun: true,
          skipTests: false,
          skipSolanaVerification: false,
          operation: "deploy",
          forceBroadcast: false,
          safetyBufferPercent: 20,
          source: {},
        },
        {
          run: async () => {
            throw new Error("command boundary must not run")
          },
          log: (message) => logs.push(message),
          setup: async () => ({ releaseCommit: "mainnet-commit" }),
          preflight: async (target) => {
            started += 1
            attempts.set(target.key, (attempts.get(target.key) ?? 0) + 1)
            if (started === 7) release()
            await allStarted
            if (target.name === "arbitrum" && attempts.get(target.key) === 1) {
              throw new Error("HTTP request failed: fetch failed")
            }
            return {
              artifactHash: `hash-${target.key}`,
              funding: {
                status: "checked" as const,
                asset: target.family === "solana" ? "SOL" : "ETH",
                decimals: target.family === "solana" ? 9 : 18,
                balance: 2n,
                estimated: 1n,
                required: 1n,
                deficit: 0n,
              },
            }
          },
          manifestPath,
          retryDelayMs: 0,
        }
      )

      assert.equal(started, 8)
      assert.equal(attempts.get("42161"), 2)
      assert.ok(
        logs.some((message) =>
          message.includes("arbitrum: transient RPC failure")
        )
      )
      const summaries = logs.filter((message) =>
        message.startsWith("Funding summary:")
      )
      assert.equal(summaries.length, 1)
      for (const network of [
        "ethereum",
        "base",
        "arbitrum",
        "optimism",
        "polygon",
        "bnb",
        "mainnet-beta",
      ]) {
        assert.match(summaries[0]!, new RegExp(network))
      }
    } finally {
      await rm(directory, { recursive: true, force: true })
    }
  }
)

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
        operation: "deploy",
        forceBroadcast: false,
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

test("upgrade advances an incomplete release without replacing unchanged completed targets", async () => {
  const directory = await mkdtemp(join(tmpdir(), "pockless-upgrade-resume-"))
  const manifestPath = join(directory, "testnet.json")
  const existing = newManifest("testnet", "previous-commit")
  existing.targets["84532"] = {
    family: "evm",
    name: "base-sepolia",
    status: "complete",
    artifactHash: "hash-84532",
    address: testUsdc,
    txHash: `0x${"1".repeat(64)}`,
    codeHash: `0x${"2".repeat(64)}`,
  }
  existing.targets.devnet = {
    family: "solana",
    name: "devnet",
    status: "failed",
    artifactHash: "previous-solana-hash",
    programId: "program-id",
  }
  await saveManifest(manifestPath, existing)
  try {
    const manifest = await runDeploy(
      {
        environment: "testnet",
        dryRun: true,
        skipTests: false,
        skipSolanaVerification: true,
        operation: "upgrade",
        forceBroadcast: false,
        safetyBufferPercent: 20,
        source: { BASE_SEPOLIA_USDC_ADDRESS: testUsdc },
      },
      {
        run: async () => {
          throw new Error("command boundary must not run")
        },
        log: () => undefined,
        setup: async () => ({ releaseCommit: "current-commit" }),
        preflight: async (target) => ({ artifactHash: `hash-${target.key}` }),
        manifestPath,
      }
    )
    assert.equal(manifest.releaseCommit, "current-commit")
    assert.equal(manifest.targets["84532"].status, "complete")
    assert.equal(manifest.targets.devnet.status, "failed")
    assert.equal(manifest.targets.devnet.artifactHash, "previous-solana-hash")
    assert.equal(manifest.targets.devnet.programId, "program-id")
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})
