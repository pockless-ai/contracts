import deployments from "../../../docs/deployments.json"
import type { Deployments } from "../../lib/types"

export const pinnedDeployments = deployments as Deployments

export function evmChainOptions() {
  return Object.entries(pinnedDeployments.evm).map(([chainId, row]) => ({
    chainId: Number(chainId),
    label: row.name,
    rpc: row.rpc,
    usdc: row.usdc,
  }))
}

export function solanaClusterOptions() {
  return Object.entries(pinnedDeployments.solana).map(([cluster, row]) => ({
    cluster,
    label: row.name,
    rpc: row.rpc,
    programId: row.programId,
    usdcMint: row.usdcMint,
  }))
}
