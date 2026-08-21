#!/usr/bin/env tsx
import { runDeploy } from "./deploy"
import { env } from "./env"
import { runImmutable } from "./immutable"
import {
  assertInteractiveMainnet,
  confirmExact,
  mainnetPhrase,
  upgradePhrase,
} from "./safety"
import type { Environment } from "./config"

type Flags = Record<string, string | boolean>

function parse(argv: string[]) {
  const command = argv[0]
  const flags: Flags = {}
  for (let index = 1; index < argv.length; index += 1) {
    const token = argv[index]
    if (!token.startsWith("--"))
      throw new Error(`unexpected argument: ${token}`)
    const key = token.slice(2)
    if (
      [
        "dry-run",
        "skip-tests",
        "skip-solana-verification",
        "force-broadcast",
        "help",
      ].includes(key)
    ) {
      flags[key] = true
      continue
    }
    const value = argv[index + 1]
    if (!value || value.startsWith("--"))
      throw new Error(`${token} requires a value`)
    flags[key] = value
    index += 1
  }
  const allowed = new Set([
    "environment",
    "dry-run",
    "skip-tests",
    "skip-solana-verification",
    "force-broadcast",
    "safety-buffer-percent",
    "help",
  ])
  for (const key of Object.keys(flags)) {
    if (!allowed.has(key)) throw new Error(`unsupported flag --${key}`)
  }
  return { command, flags }
}

function usage() {
  console.log(`Secure Pockless contract deployment

Usage:
  yarn deploy --environment testnet|mainnet [--dry-run]
  yarn upgrade --environment testnet|mainnet [--dry-run]
  yarn immutable --environment mainnet

Testnet-only flags:
  --skip-tests
  --skip-solana-verification
  --force-broadcast                Re-broadcast an incomplete recorded target

Options:
  --safety-buffer-percent <integer>  Funding buffer (default 20)
`)
}

async function main() {
  const { command, flags } = parse(process.argv.slice(2))
  if (flags.help) return usage()
  if (command !== "deploy" && command !== "upgrade" && command !== "immutable") {
    usage()
    throw new Error("command must be deploy, upgrade, or immutable")
  }
  const environment = flags.environment
  if (environment !== "testnet" && environment !== "mainnet") {
    throw new Error("--environment must be testnet or mainnet")
  }
  assertInteractiveMainnet(environment)
  if (command === "immutable") {
    if (environment !== "mainnet") {
      throw new Error("immutable is restricted to mainnet")
    }
    console.log("EVM SessionSpend7702 deployments are already immutable.")
    await runImmutable(environment, env)
    return
  }
  if (command === "upgrade" && flags["force-broadcast"] === true) {
    throw new Error("upgrade does not accept --force-broadcast")
  }
  const dryRun = flags["dry-run"] === true
  if (environment === "mainnet" && !dryRun) {
    await confirmExact(
      command === "upgrade"
        ? upgradePhrase(environment)
        : mainnetPhrase(environment)
    )
  }
  const safetyBufferPercent = Number(flags["safety-buffer-percent"] ?? "20")
  if (
    !Number.isInteger(safetyBufferPercent) ||
    safetyBufferPercent < 0 ||
    safetyBufferPercent > 500
  ) {
    throw new Error("--safety-buffer-percent must be an integer from 0 to 500")
  }
  await runDeploy({
    environment: environment as Environment,
    dryRun,
    skipTests: flags["skip-tests"] === true,
    skipSolanaVerification: flags["skip-solana-verification"] === true,
    operation: command,
    forceBroadcast: flags["force-broadcast"] === true,
    safetyBufferPercent,
    source: env,
  })
  console.log(
    dryRun
      ? "Preflight complete; no transactions were signed or broadcast."
      : "Deployment complete."
  )
}

main().catch((error) => {
  let message = error instanceof Error ? error.message : String(error)
  for (const value of Object.values(env)) {
    if (value) message = message.split(value).join("<redacted>")
  }
  console.error(message)
  process.exitCode = 1
})
