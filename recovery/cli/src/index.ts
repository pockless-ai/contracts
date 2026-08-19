#!/usr/bin/env tsx
import { Keypair } from "@solana/web3.js"
import { Connection, PublicKey } from "@solana/web3.js"
import { createWalletClient, http, type Address, type Hex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import {
  createEvmClient,
  listEvmSessions,
  revokeEvmSession,
  withdrawEvmToken,
} from "../../lib/evm"
import {
  closeSolanaStrategy,
  listSolanaStrategies,
  revokeSolanaStrategy,
  withdrawSolanaAsset,
} from "../../lib/solana"
import { loadDeployments } from "../../lib/deployments-node"
import { formatUsdc, parseStrategyId } from "../../lib/types"
import { parseArgs } from "./parse-args"

function usage() {
  console.log(`pockless-recovery — owner recovery CLI

Usage:
  pockless-recovery list-strategies [options]
  pockless-recovery revoke [options]
  pockless-recovery withdraw [options]
  pockless-recovery close [options]

Global options:
  --chain evm|solana          Chain family (required)
  --rpc <url>                 Public RPC URL (required)
  --owner <address>           Owner wallet address (required)
  --private-key <hex>         Signer key for write commands (EVM 0x…, Solana base58 or hex)
  --chain-id <id>             EVM chain id (default: from deployments)
  --program-id <pubkey>       Solana program id (default: from deployments)
  --deployments <path>        deployments.json path
  --help                      Show this help

list-strategies:
  Lists EVM strategy sessions or Solana strategy vaults for --owner.

revoke:
  --strategy-id <hex32>       Strategy identifier
  --session-key <address>     EVM session key (EVM only)

withdraw:
  --strategy-id <hex32>       Strategy id (Solana) or label (EVM uses token transfer)
  --token <address>           EVM ERC-20 token address
  --mint <pubkey>             Solana token mint
  --amount <integer>          Token amount in base units
  --to <address>              EVM recipient (defaults to --owner)

close:
  --strategy-id <hex32>       Solana strategy to close (requires deployed_usdc == 0)
`)
}

async function main() {
  const { command, flags } = parseArgs(process.argv.slice(2))

  if (flags.help || !command) {
    usage()
    process.exit(command ? 0 : 1)
  }

  const chain = flags.chain
  if (chain !== "evm" && chain !== "solana") {
    throw new Error("--chain must be evm or solana")
  }

  const rpc = flags.rpc
  const owner = flags.owner
  if (!rpc || !owner) {
    throw new Error("--rpc and --owner are required")
  }

  const deployments = loadDeployments(flags.deployments)

  if (command === "list-strategies") {
    if (chain === "evm") {
      const chainId = Number(flags["chain-id"] ?? "1")
      const client = createEvmClient(rpc, chainId)
      const sessions = await listEvmSessions(client, owner as Address)
      if (sessions.length === 0) {
        console.log("No EVM sessions found.")
        return
      }
      for (const session of sessions) {
        console.log(
          JSON.stringify(
            {
              strategyId: session.strategyId,
              sessionKey: session.sessionKey,
              limitUsdc: formatUsdc(session.limitUsdc),
              capacityUsdc: formatUsdc(session.capacityUsdc),
              deployedUsdc: formatUsdc(session.deployedUsdc),
              expiresAt: session.expiresAt.toString(),
              nonce: session.nonce.toString(),
              revoked: session.revoked,
            },
            null,
            2
          )
        )
      }
      return
    }

    const programId = new PublicKey(
      flags["program-id"] ??
        deployments.solana["mainnet-beta"]?.programId ??
        PublicKey.default.toBase58()
    )
    const connection = new Connection(rpc, "confirmed")
    const strategies = await listSolanaStrategies({
      connection,
      programId,
      owner: new PublicKey(owner),
    })

    if (strategies.length === 0) {
      console.log("No Solana strategies found.")
      return
    }

    for (const strategy of strategies) {
      console.log(
        JSON.stringify(
          {
            pubkey: strategy.pubkey,
            strategyId: strategy.strategyId,
            session: strategy.session,
            limitUsdc: formatUsdc(strategy.limitUsdc),
            capacityUsdc: formatUsdc(strategy.capacityUsdc),
            deployedUsdc: formatUsdc(strategy.deployedUsdc),
            expiresAt: strategy.expiresAt.toString(),
            nonce: strategy.nonce.toString(),
            revoked: strategy.revoked,
          },
          null,
          2
        )
      )
    }
    return
  }

  const privateKey = flags["private-key"]
  if (!privateKey) {
    throw new Error(`--private-key is required for ${command}`)
  }

  if (command === "revoke") {
    const strategyId = flags["strategy-id"] as Hex | undefined
    if (!strategyId) throw new Error("--strategy-id is required")

    if (chain === "evm") {
      const sessionKey = flags["session-key"] as Address | undefined
      if (!sessionKey) throw new Error("--session-key is required for EVM revoke")

      const chainId = Number(flags["chain-id"] ?? "1")
      const account = privateKeyToAccount(privateKey as Hex)
      const wallet = createWalletClient({
        account,
        chain: {
          id: chainId,
          name: `chain-${chainId}`,
          nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
          rpcUrls: { default: { http: [rpc] } },
        },
        transport: http(rpc),
      })

      const hash = await revokeEvmSession({
        wallet,
        owner: owner as Address,
        strategyId,
        sessionKey,
      })
      console.log(`revoke submitted: ${hash}`)
      return
    }

    const programId = new PublicKey(
      flags["program-id"] ??
        deployments.solana["mainnet-beta"].programId
    )
    const keypair = loadSolanaKeypair(privateKey)
    const connection = new Connection(rpc, "confirmed")
    const signature = await revokeSolanaStrategy({
      connection,
      programId,
      owner: keypair,
      strategyId: parseStrategyId(strategyId),
    })
    console.log(`revoke confirmed: ${signature}`)
    return
  }

  if (command === "withdraw") {
    if (chain === "evm") {
      const token = flags.token as Address | undefined
      const amount = flags.amount ? BigInt(flags.amount) : undefined
      if (!token || amount === undefined) {
        throw new Error("--token and --amount are required for EVM withdraw")
      }

      const chainId = Number(flags["chain-id"] ?? "1")
      const account = privateKeyToAccount(privateKey as Hex)
      const wallet = createWalletClient({
        account,
        chain: {
          id: chainId,
          name: `chain-${chainId}`,
          nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
          rpcUrls: { default: { http: [rpc] } },
        },
        transport: http(rpc),
      })

      const hash = await withdrawEvmToken({
        wallet,
        owner: owner as Address,
        token,
        to: (flags.to as Address | undefined) ?? (owner as Address),
        amount,
      })
      console.log(`withdraw submitted: ${hash}`)
      return
    }

    const strategyId = flags["strategy-id"]
    const mint = flags.mint
    const amount = flags.amount ? BigInt(flags.amount) : undefined
    if (!strategyId || !mint || amount === undefined) {
      throw new Error(
        "--strategy-id, --mint, and --amount are required for Solana withdraw"
      )
    }

    const programId = new PublicKey(
      flags["program-id"] ??
        deployments.solana["mainnet-beta"].programId
    )
    const keypair = loadSolanaKeypair(privateKey)
    const connection = new Connection(rpc, "confirmed")
    const signature = await withdrawSolanaAsset({
      connection,
      programId,
      owner: keypair,
      strategyId: parseStrategyId(strategyId),
      mint: new PublicKey(mint),
      amount,
    })
    console.log(`withdraw confirmed: ${signature}`)
    return
  }

  if (command === "close") {
    if (chain === "evm") {
      throw new Error("close is not supported on EVM — revoke sessions and transfer balances")
    }

    const strategyId = flags["strategy-id"]
    if (!strategyId) throw new Error("--strategy-id is required")

    const programId = new PublicKey(
      flags["program-id"] ??
        deployments.solana["mainnet-beta"].programId
    )
    const keypair = loadSolanaKeypair(privateKey)
    const connection = new Connection(rpc, "confirmed")
    const signature = await closeSolanaStrategy({
      connection,
      programId,
      owner: keypair,
      strategyId: parseStrategyId(strategyId),
    })
    console.log(`close confirmed: ${signature}`)
    return
  }

  throw new Error(`unknown command: ${command}`)
}

function loadSolanaKeypair(raw: string): Keypair {
  if (raw.startsWith("0x")) {
    return Keypair.fromSecretKey(Buffer.from(raw.slice(2), "hex"))
  }
  try {
    return Keypair.fromSecretKey(Buffer.from(raw, "base64"))
  } catch {
    return Keypair.fromSecretKey(Buffer.from(JSON.parse(raw)))
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error)
  process.exit(1)
})
