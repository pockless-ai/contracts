import {
  encodeCloseStrategy,
  encodeInitStrategy,
  encodeInitWallet,
  encodeRevoke,
  JUPITER_V6_PROGRAM_ID,
  strategyPda,
  walletPda,
} from "@pockless/protocol-sdk"
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  type Signer,
} from "@solana/web3.js"
import { readFile } from "node:fs/promises"
import { keccak256, toBytes, type Hex } from "viem"
import { checked, runCommand } from "../../../src/command"
import type { SmokeConfig } from "./config"

const STRATEGY_ACCOUNT_LEN = 138

export type DecodedStrategy = {
  strategyId: Hex
  session: string
  limitUsdc: bigint
  capacityUsdc: bigint
  deployedUsdc: bigint
  expiresAt: bigint
  nonce: bigint
  revoked: boolean
}

export async function loadSolanaOwner(path: string) {
  const secret = JSON.parse(await readFile(path, "utf8")) as number[]
  return Keypair.fromSecretKey(Uint8Array.from(secret))
}

export function solanaConnection(rpc: string) {
  return new Connection(rpc, "confirmed")
}

export function uniqueSolanaStrategyId(label: string): Uint8Array {
  const hash = keccak256(
    toBytes(`${label}-${Date.now()}-${Math.random()}`)
  )
  return Buffer.from(hash.slice(2), "hex")
}

function decodeStrategy(data: Buffer): DecodedStrategy {
  return {
    strategyId: `0x${data.subarray(0, 32).toString("hex")}` as Hex,
    session: new PublicKey(data.subarray(64, 96)).toBase58(),
    limitUsdc: data.readBigUInt64LE(96),
    capacityUsdc: data.readBigUInt64LE(104),
    deployedUsdc: data.readBigUInt64LE(112),
    expiresAt: data.readBigInt64LE(120),
    nonce: data.readBigUInt64LE(128),
    revoked: data[136] === 1,
  }
}

export async function assertSolanaDeployment(config: SmokeConfig["solana"]) {
  const owner = await loadSolanaOwner(config.ownerKeypairPath)
  const connection = solanaConnection(config.rpc)
  const programId = new PublicKey(config.programId)
  const program = await connection.getAccountInfo(programId)
  if (!program?.executable) {
    throw new Error("Solana program account is not executable on devnet")
  }

  const result = await checked(runCommand, "solana", [
    "program",
    "show",
    config.programId,
    "--url",
    config.rpc,
    "--keypair",
    config.ownerKeypairPath,
  ])
  const output = result.stdout
  const authorityMatch = output.match(/Authority:\s+(\S+)/)
  if (!authorityMatch?.[1]) {
    throw new Error("Solana program is missing an upgrade authority")
  }
  if (authorityMatch[1] !== owner.publicKey.toBase58()) {
    throw new Error("Solana program upgrade authority does not match deployer")
  }
}

async function sendInstructions(input: {
  connection: Connection
  payer: PublicKey
  signers: Signer[]
  instructions: TransactionInstruction[]
}) {
  const transaction = new Transaction().add(...input.instructions)
  transaction.feePayer = input.payer
  transaction.recentBlockhash = (
    await input.connection.getLatestBlockhash()
  ).blockhash
  transaction.sign(...input.signers)
  const signature = await input.connection.sendRawTransaction(
    transaction.serialize()
  )
  await input.connection.confirmTransaction(signature, "confirmed")
  return signature
}

export async function ensureSolanaWallet(input: {
  connection: Connection
  config: SmokeConfig["solana"]
  owner: Keypair
}) {
  const programId = new PublicKey(input.config.programId)
  const usdcMint = new PublicKey(input.config.usdcMint)
  const [wallet] = walletPda(programId, input.owner.publicKey)
  const existing = await input.connection.getAccountInfo(wallet)
  if (existing) return wallet

  await sendInstructions({
    connection: input.connection,
    payer: input.owner.publicKey,
    signers: [input.owner],
    instructions: [
      new TransactionInstruction({
        programId,
        keys: [
          { pubkey: input.owner.publicKey, isSigner: true, isWritable: true },
          { pubkey: wallet, isSigner: false, isWritable: true },
          { pubkey: usdcMint, isSigner: false, isWritable: false },
          {
            pubkey: new PublicKey(JUPITER_V6_PROGRAM_ID),
            isSigner: false,
            isWritable: false,
          },
          { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        data: encodeInitWallet(),
      }),
    ],
  })
  return wallet
}

export async function initSolanaStrategy(input: {
  connection: Connection
  config: SmokeConfig["solana"]
  owner: Keypair
  strategyId: Uint8Array
  session: Keypair
  limitUsdc?: bigint
  expiresAt?: bigint
}) {
  const programId = new PublicKey(input.config.programId)
  const [wallet] = walletPda(programId, input.owner.publicKey)
  const [strategy] = strategyPda(
    programId,
    input.owner.publicKey,
    input.strategyId
  )

  await sendInstructions({
    connection: input.connection,
    payer: input.owner.publicKey,
    signers: [input.owner],
    instructions: [
      new TransactionInstruction({
        programId,
        keys: [
          { pubkey: input.owner.publicKey, isSigner: true, isWritable: true },
          { pubkey: wallet, isSigner: false, isWritable: false },
          { pubkey: strategy, isSigner: false, isWritable: true },
          { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        data: encodeInitStrategy({
          strategyId: input.strategyId,
          session: input.session.publicKey,
          limitUsdc: input.limitUsdc ?? 1_000_000n,
          expiresAt:
            input.expiresAt ??
            BigInt(Math.floor(Date.now() / 1000) + 3600),
        }),
      }),
    ],
  })

  return strategy
}

export async function readSolanaStrategy(input: {
  connection: Connection
  strategy: PublicKey
}) {
  const account = await input.connection.getAccountInfo(input.strategy)
  if (!account || account.data.length < STRATEGY_ACCOUNT_LEN) {
    throw new Error("Solana strategy account is missing")
  }
  return decodeStrategy(Buffer.from(account.data))
}

export async function revokeSolanaStrategy(input: {
  connection: Connection
  config: SmokeConfig["solana"]
  owner: Keypair
  strategyId: Uint8Array
}) {
  const programId = new PublicKey(input.config.programId)
  const [strategy] = strategyPda(
    programId,
    input.owner.publicKey,
    input.strategyId
  )
  return sendInstructions({
    connection: input.connection,
    payer: input.owner.publicKey,
    signers: [input.owner],
    instructions: [
      new TransactionInstruction({
        programId,
        keys: [
          { pubkey: input.owner.publicKey, isSigner: true, isWritable: false },
          { pubkey: strategy, isSigner: false, isWritable: true },
        ],
        data: encodeRevoke(),
      }),
    ],
  })
}

export async function closeSolanaStrategy(input: {
  connection: Connection
  config: SmokeConfig["solana"]
  owner: Keypair
  strategyId: Uint8Array
}) {
  const programId = new PublicKey(input.config.programId)
  const [wallet] = walletPda(programId, input.owner.publicKey)
  const [strategy] = strategyPda(
    programId,
    input.owner.publicKey,
    input.strategyId
  )
  return sendInstructions({
    connection: input.connection,
    payer: input.owner.publicKey,
    signers: [input.owner],
    instructions: [
      new TransactionInstruction({
        programId,
        keys: [
          { pubkey: input.owner.publicKey, isSigner: true, isWritable: true },
          { pubkey: wallet, isSigner: false, isWritable: false },
          { pubkey: strategy, isSigner: false, isWritable: true },
          { pubkey: input.owner.publicKey, isSigner: false, isWritable: true },
        ],
        data: encodeCloseStrategy(),
      }),
    ],
  })
}
