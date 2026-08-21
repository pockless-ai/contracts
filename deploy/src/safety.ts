import { createInterface } from "node:readline/promises"

export function mainnetPhrase(environment: string) {
  return `DEPLOY POCKLESS ${environment.toUpperCase()}`
}

export function upgradePhrase(environment: string) {
  return `UPGRADE POCKLESS ${environment.toUpperCase()}`
}

export function immutablePhrase(cluster: string, programId: string) {
  return `MAKE SOLANA ${cluster} ${programId} PERMANENTLY IMMUTABLE`
}

export function assertInteractiveMainnet(
  environment: string,
  stdin: NodeJS.ReadStream = process.stdin,
  stdout: NodeJS.WriteStream = process.stdout
) {
  if (environment === "mainnet" && (!stdin.isTTY || !stdout.isTTY)) {
    throw new Error("mainnet operations require an attached interactive TTY")
  }
}

export async function confirmExact(
  phrase: string,
  stdin: NodeJS.ReadStream = process.stdin,
  stdout: NodeJS.WriteStream = process.stdout
) {
  if (!stdin.isTTY || !stdout.isTTY) {
    throw new Error("confirmation requires an attached interactive TTY")
  }
  const reader = createInterface({ input: stdin, output: stdout })
  try {
    const answer = await reader.question(`Type exactly: ${phrase}\n> `)
    if (answer !== phrase) throw new Error("confirmation phrase did not match")
  } finally {
    reader.close()
  }
}
