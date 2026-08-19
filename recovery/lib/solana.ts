import {
  encodeCloseStrategy,
  encodeRevoke,
  encodeWithdrawAsset,
  strategyPda,
  vaultPda,
  assetPda,
  walletPda,
} from "@pockless/protocol-sdk"
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token"
import {
  Connection,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  type Signer,
} from "@solana/web3.js"
import { hexStrategyId, type SolanaStrategy } from "./types"

const STRATEGY_ACCOUNT_LEN = 138
const STRATEGY_OWNER_OFFSET = 32

function decodeStrategyAccount(
  pubkey: PublicKey,
  data: Buffer
): SolanaStrategy | null {
  if (data.length < STRATEGY_ACCOUNT_LEN) return null

  const strategyId = hexStrategyId(data.subarray(0, 32))
  const session = new PublicKey(data.subarray(64, 96))
  const limitUsdc = data.readBigUInt64LE(96)
  const capacityUsdc = data.readBigUInt64LE(104)
  const deployedUsdc = data.readBigUInt64LE(112)
  const expiresAt = data.readBigInt64LE(120)
  const nonce = data.readBigUInt64LE(128)
  const revoked = data[136] === 1

  return {
    pubkey: pubkey.toBase58(),
    strategyId,
    session: session.toBase58(),
    limitUsdc,
    capacityUsdc,
    deployedUsdc,
    expiresAt,
    nonce,
    revoked,
  }
}

export async function listSolanaStrategies(input: {
  connection: Connection
  programId: PublicKey
  owner: PublicKey
}): Promise<SolanaStrategy[]> {
  const accounts = await input.connection.getProgramAccounts(input.programId, {
    filters: [
      { dataSize: STRATEGY_ACCOUNT_LEN },
      { memcmp: { offset: STRATEGY_OWNER_OFFSET, bytes: input.owner.toBase58() } },
    ],
  })

  return accounts
    .map(({ pubkey, account }) =>
      decodeStrategyAccount(pubkey, Buffer.from(account.data))
    )
    .filter((row): row is SolanaStrategy => row !== null)
}

export function buildRevokeInstruction(input: {
  programId: PublicKey
  owner: PublicKey
  strategyId: Uint8Array
}) {
  const [strategy] = strategyPda(input.programId, input.owner, input.strategyId)

  return new TransactionInstruction({
    programId: input.programId,
    keys: [
      { pubkey: input.owner, isSigner: true, isWritable: false },
      { pubkey: strategy, isSigner: false, isWritable: true },
    ],
    data: encodeRevoke(),
  })
}

export function buildWithdrawInstruction(input: {
  programId: PublicKey
  owner: PublicKey
  strategyId: Uint8Array
  mint: PublicKey
  amount: bigint
}) {
  const [wallet] = walletPda(input.programId, input.owner)
  const [strategy] = strategyPda(input.programId, input.owner, input.strategyId)
  const [vaultAuthority] = vaultPda(input.programId, strategy)
  const strategyVault = getAssociatedTokenAddressSync(
    input.mint,
    vaultAuthority,
    true
  )
  const ownerToken = getAssociatedTokenAddressSync(input.mint, input.owner)
  const [assetAccount] = assetPda(input.programId, strategy, input.mint)

  return new TransactionInstruction({
    programId: input.programId,
    keys: [
      { pubkey: input.owner, isSigner: true, isWritable: false },
      { pubkey: wallet, isSigner: false, isWritable: false },
      { pubkey: strategy, isSigner: false, isWritable: true },
      { pubkey: vaultAuthority, isSigner: false, isWritable: false },
      { pubkey: strategyVault, isSigner: false, isWritable: true },
      { pubkey: ownerToken, isSigner: false, isWritable: true },
      { pubkey: input.mint, isSigner: false, isWritable: false },
      { pubkey: assetAccount, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: encodeWithdrawAsset(input.amount),
  })
}

export function buildCloseInstruction(input: {
  programId: PublicKey
  owner: PublicKey
  strategyId: Uint8Array
}) {
  const [wallet] = walletPda(input.programId, input.owner)
  const [strategy] = strategyPda(input.programId, input.owner, input.strategyId)

  return new TransactionInstruction({
    programId: input.programId,
    keys: [
      { pubkey: input.owner, isSigner: true, isWritable: true },
      { pubkey: wallet, isSigner: false, isWritable: false },
      { pubkey: strategy, isSigner: false, isWritable: true },
      { pubkey: input.owner, isSigner: false, isWritable: true },
    ],
    data: encodeCloseStrategy(),
  })
}

export async function sendSolanaInstructions(input: {
  connection: Connection
  payer: PublicKey
  signers: Signer[]
  instructions: TransactionInstruction[]
}) {
  const tx = await buildSolanaTransaction(input.connection, input.payer, input.instructions)
  tx.sign(...input.signers)
  const signature = await input.connection.sendRawTransaction(tx.serialize())
  await input.connection.confirmTransaction(signature, "confirmed")
  return signature
}

export async function sendSolanaWithWallet(input: {
  connection: Connection
  payer: PublicKey
  signTransaction: (tx: Transaction) => Promise<Transaction>
  instructions: TransactionInstruction[]
}) {
  const tx = await buildSolanaTransaction(input.connection, input.payer, input.instructions)
  const signed = await input.signTransaction(tx)
  const signature = await input.connection.sendRawTransaction(signed.serialize())
  await input.connection.confirmTransaction(signature, "confirmed")
  return signature
}

async function buildSolanaTransaction(
  connection: Connection,
  payer: PublicKey,
  instructions: TransactionInstruction[]
) {
  const tx = new Transaction().add(...instructions)
  tx.feePayer = payer
  tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash
  return tx
}

export async function revokeSolanaStrategy(input: {
  connection: Connection
  programId: PublicKey
  owner: Signer
  strategyId: Uint8Array
}) {
  return sendSolanaInstructions({
    connection: input.connection,
    payer: input.owner.publicKey,
    signers: [input.owner],
    instructions: [
      buildRevokeInstruction({
        programId: input.programId,
        owner: input.owner.publicKey,
        strategyId: input.strategyId,
      }),
    ],
  })
}

export async function withdrawSolanaAsset(input: {
  connection: Connection
  programId: PublicKey
  owner: Signer
  strategyId: Uint8Array
  mint: PublicKey
  amount: bigint
}) {
  return sendSolanaInstructions({
    connection: input.connection,
    payer: input.owner.publicKey,
    signers: [input.owner],
    instructions: [
      buildWithdrawInstruction({
        programId: input.programId,
        owner: input.owner.publicKey,
        strategyId: input.strategyId,
        mint: input.mint,
        amount: input.amount,
      }),
    ],
  })
}

export async function closeSolanaStrategy(input: {
  connection: Connection
  programId: PublicKey
  owner: Signer
  strategyId: Uint8Array
}) {
  return sendSolanaInstructions({
    connection: input.connection,
    payer: input.owner.publicKey,
    signers: [input.owner],
    instructions: [
      buildCloseInstruction({
        programId: input.programId,
        owner: input.owner.publicKey,
        strategyId: input.strategyId,
      }),
    ],
  })
}
