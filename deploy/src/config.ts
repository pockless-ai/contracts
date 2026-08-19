import { getAddress, isAddress, zeroAddress, type Address } from "viem"

export type Environment = "testnet" | "mainnet"

export type EvmTarget = {
  family: "evm"
  key: string
  name: string
  chainId: number
  rpcEnv: string
  usdc: Address
  usdcDecimals: number
}

export type SolanaTarget = {
  family: "solana"
  key: string
  name: string
  cluster: "devnet" | "mainnet-beta"
  rpcEnv: string
  usdcMint: string
}

export type Target = EvmTarget | SolanaTarget

const mainnetEvm = [
  [
    "ethereum",
    1,
    "ETHEREUM_RPC_URL",
    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    6,
  ],
  [
    "base",
    8453,
    "BASE_RPC_URL",
    "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
    6,
  ],
  [
    "arbitrum",
    42161,
    "ARBITRUM_RPC_URL",
    "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
    6,
  ],
  [
    "optimism",
    10,
    "OPTIMISM_RPC_URL",
    "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
    6,
  ],
  [
    "polygon",
    137,
    "POLYGON_RPC_URL",
    "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359",
    6,
  ],
  ["bnb", 56, "BNB_RPC_URL", "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d", 18],
] as const

function evm(
  name: string,
  chainId: number,
  rpcEnv: string,
  usdc: string,
  usdcDecimals: number
): EvmTarget {
  if (!isAddress(usdc) || getAddress(usdc) === zeroAddress) {
    throw new Error(`invalid USDC address for ${name}`)
  }
  return {
    family: "evm",
    key: String(chainId),
    name,
    chainId,
    rpcEnv,
    usdc: getAddress(usdc),
    usdcDecimals,
  }
}

export function loadTargets(
  environment: Environment,
  source: Record<string, string | undefined>
): Target[] {
  if (environment === "testnet") {
    const testUsdc = source.BASE_SEPOLIA_USDC_ADDRESS?.trim()
    if (!testUsdc) {
      throw new Error("BASE_SEPOLIA_USDC_ADDRESS is required for testnet")
    }
    return [
      evm("base-sepolia", 84532, "BASE_SEPOLIA_RPC_URL", testUsdc, 6),
      {
        family: "solana",
        key: "devnet",
        name: "devnet",
        cluster: "devnet",
        rpcEnv: "SOLANA_DEVNET_RPC_URL",
        usdcMint: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",
      },
    ]
  }

  return [
    ...mainnetEvm.map(([name, id, rpc, usdc, decimals]) =>
      evm(name, id, rpc, usdc, decimals)
    ),
    {
      family: "solana",
      key: "mainnet-beta",
      name: "mainnet-beta",
      cluster: "mainnet-beta",
      rpcEnv: "SOLANA_MAINNET_RPC_URL",
      usdcMint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    },
  ]
}

export function requiredRpc(
  target: Target,
  source: Record<string, string | undefined>
) {
  const value = source[target.rpcEnv]?.trim()
  if (!value) throw new Error(`${target.rpcEnv} is required`)
  return value
}
