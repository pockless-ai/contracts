import {
  encodeSessionRevoke,
  sessionSpend7702Abi,
} from "@pockless/protocol-sdk"
import {
  createPublicClient,
  encodeFunctionData,
  erc20Abi,
  http,
  type Address,
  type Hex,
  type PublicClient,
  type WalletClient,
} from "viem"
import type { EvmSession } from "./types"

const recoveryViewAbi = [
  ...sessionSpend7702Abi,
  {
    type: "function",
    name: "strategyCount",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "uint256" }],
  },
  {
    type: "function",
    name: "strategyAt",
    stateMutability: "view",
    inputs: [{ name: "index", type: "uint256" }],
    outputs: [{ name: "", type: "bytes32" }],
  },
  {
    type: "function",
    name: "sessionCount",
    stateMutability: "view",
    inputs: [{ name: "strategyId", type: "bytes32" }],
    outputs: [{ name: "", type: "uint256" }],
  },
  {
    type: "function",
    name: "sessionAt",
    stateMutability: "view",
    inputs: [
      { name: "strategyId", type: "bytes32" },
      { name: "index", type: "uint256" },
    ],
    outputs: [{ name: "", type: "address" }],
  },
  {
    type: "function",
    name: "sessionOf",
    stateMutability: "view",
    inputs: [
      { name: "strategyId", type: "bytes32" },
      { name: "key", type: "address" },
    ],
    outputs: [
      {
        name: "",
        type: "tuple",
        components: [
          { name: "limitUsdc", type: "uint128" },
          { name: "capacityUsdc", type: "uint128" },
          { name: "deployedUsdc", type: "uint128" },
          { name: "expiresAt", type: "uint64" },
          { name: "nonce", type: "uint64" },
          { name: "revoked", type: "bool" },
          { name: "exists", type: "bool" },
        ],
      },
    ],
  },
] as const

export function createEvmClient(rpc: string, chainId: number) {
  return createPublicClient({
    transport: http(rpc),
    chain: {
      id: chainId,
      name: `chain-${chainId}`,
      nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
      rpcUrls: { default: { http: [rpc] } },
    },
  })
}

export async function listEvmSessions(
  client: PublicClient,
  owner: Address
): Promise<EvmSession[]> {
  const strategyCount = await client.readContract({
    address: owner,
    abi: recoveryViewAbi,
    functionName: "strategyCount",
  })

  const sessions: EvmSession[] = []

  for (let i = 0n; i < strategyCount; i++) {
    const strategyId = await client.readContract({
      address: owner,
      abi: recoveryViewAbi,
      functionName: "strategyAt",
      args: [i],
    })

    const sessionCount = await client.readContract({
      address: owner,
      abi: recoveryViewAbi,
      functionName: "sessionCount",
      args: [strategyId],
    })

    for (let j = 0n; j < sessionCount; j++) {
      const sessionKey = await client.readContract({
        address: owner,
        abi: recoveryViewAbi,
        functionName: "sessionAt",
        args: [strategyId, j],
      })

      const session = await client.readContract({
        address: owner,
        abi: recoveryViewAbi,
        functionName: "sessionOf",
        args: [strategyId, sessionKey],
      })

      if (!session.exists) continue

      sessions.push({
        strategyId,
        sessionKey,
        limitUsdc: BigInt(session.limitUsdc),
        capacityUsdc: BigInt(session.capacityUsdc),
        deployedUsdc: BigInt(session.deployedUsdc),
        expiresAt: BigInt(session.expiresAt),
        nonce: BigInt(session.nonce),
        revoked: session.revoked,
      })
    }
  }

  return sessions
}

export async function revokeEvmSession(input: {
  wallet: WalletClient
  owner: Address
  strategyId: Hex
  sessionKey: Address
}) {
  const data = encodeSessionRevoke({
    strategyId: input.strategyId,
    sessionAddress: input.sessionKey,
  })

  const hash = await input.wallet.sendTransaction({
    account: input.owner,
    chain: input.wallet.chain,
    to: input.owner,
    data,
  })

  return hash
}

export async function withdrawEvmToken(input: {
  wallet: WalletClient
  owner: Address
  token: Address
  to: Address
  amount: bigint
}) {
  const data = encodeFunctionData({
    abi: erc20Abi,
    functionName: "transfer",
    args: [input.to, input.amount],
  })

  const hash = await input.wallet.sendTransaction({
    account: input.owner,
    chain: input.wallet.chain,
    to: input.token,
    data,
  })

  return hash
}

export async function readErc20Balance(
  client: PublicClient,
  token: Address,
  owner: Address
) {
  return client.readContract({
    address: token,
    abi: erc20Abi,
    functionName: "balanceOf",
    args: [owner],
  })
}
