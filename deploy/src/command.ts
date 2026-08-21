import { spawn } from "node:child_process"
import { existsSync } from "node:fs"
import { homedir } from "node:os"
import { join } from "node:path"

export type CommandResult = { stdout: string; stderr: string; code: number }
export type CommandOptions = {
  cwd?: string
  env?: NodeJS.ProcessEnv
  interactive?: boolean
}
export type RunCommand = (
  executable: string,
  args: readonly string[],
  options?: CommandOptions
) => Promise<CommandResult>

const SECRET_FLAGS = new Set([
  "--account",
  "--keystore",
  "--keypair",
  "--fee-payer",
  "--upgrade-authority",
  "--program-id",
  "--from",
  "--password",
  "--etherscan-api-key",
  "--rpc-url",
  "--url",
  "-u",
])

export function redactArgs(args: readonly string[]): string[] {
  let redactNext = false
  return args.map((arg) => {
    if (redactNext) {
      redactNext = false
      return "<redacted>"
    }
    if (SECRET_FLAGS.has(arg)) {
      redactNext = true
      return arg
    }
    return arg.replace(/(https?:\/\/)([^/@\s]+)@/g, "$1<redacted>@")
  })
}

function resolvedExecutable(executable: string) {
  const home = homedir()
  const standardPaths: Record<string, string> = {
    forge: join(home, ".foundry/bin/forge"),
    cast: join(home, ".foundry/bin/cast"),
    cargo: join(home, ".cargo/bin/cargo"),
    rustup: join(home, ".cargo/bin/rustup"),
    "solana-verify": join(home, ".cargo/bin/solana-verify"),
    solana: join(
      home,
      ".local/share/solana/install/active_release/bin/solana"
    ),
    "solana-keygen": join(
      home,
      ".local/share/solana/install/active_release/bin/solana-keygen"
    ),
  }
  const candidate = standardPaths[executable]
  return candidate && existsSync(candidate) ? candidate : executable
}

export const runCommand: RunCommand = (executable, args, options = {}) =>
  new Promise((resolve, reject) => {
    const child = spawn(resolvedExecutable(executable), [...args], {
      cwd: options.cwd,
      env: options.env,
      shell: false,
      stdio: [options.interactive ? "inherit" : "ignore", "pipe", "pipe"],
    })
    let stdout = ""
    let stderr = ""
    child.stdout.on("data", (data) => {
      stdout += String(data)
      if (options.interactive) process.stdout.write(data)
    })
    child.stderr.on("data", (data) => {
      stderr += String(data)
      if (options.interactive) process.stderr.write(data)
    })
    child.on("error", (error) =>
      reject(
        new Error(
          `failed to start ${executable} ${redactArgs(args).join(" ")}: ${error.message}`
        )
      )
    )
    child.on("close", (code) => resolve({ stdout, stderr, code: code ?? 1 }))
  })

export async function checked(
  run: RunCommand,
  executable: string,
  args: readonly string[],
  options?: CommandOptions
) {
  const result = await run(executable, args, options)
  if (result.code !== 0) {
    let detail = `${result.stderr}\n${result.stdout}`.trim()
    for (let index = 0; index < args.length - 1; index += 1) {
      if (SECRET_FLAGS.has(args[index])) {
        detail = detail.split(args[index + 1]).join("<redacted>")
      }
    }
    throw new Error(
      `${executable} ${redactArgs(args).join(" ")} failed (${result.code})${
        detail ? `: ${detail}` : ""
      }`
    )
  }
  return result
}

export function evmDeployArgs(input: {
  rpc: string
  account: string
  sender: string
  usdc: string
}) {
  return [
    "create",
    "src/SessionSpend7702.sol:SessionSpend7702",
    "--root",
    ".",
    "--rpc-url",
    input.rpc,
    "--account",
    input.account,
    "--from",
    input.sender,
    "--broadcast",
    "--json",
    "--constructor-args",
    input.usdc,
  ]
}

export function solanaDeployArgs(input: {
  artifact: string
  rpc: string
  feePayer: string
  authority: string
  programKeypair: string
}) {
  return [
    "program",
    "deploy",
    input.artifact,
    "--url",
    input.rpc,
    "--keypair",
    input.feePayer,
    "--fee-payer",
    input.feePayer,
    "--upgrade-authority",
    input.authority,
    "--program-id",
    input.programKeypair,
    "--output",
    "json",
  ]
}
