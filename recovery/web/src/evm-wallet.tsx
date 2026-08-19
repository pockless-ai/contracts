import { WagmiProvider, createConfig, http, useAccount, useConnect, useDisconnect, useWalletClient } from "wagmi"
import { injected } from "wagmi/connectors"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import type { ReactNode } from "react"
import { evmChainOptions } from "./deployments"

const chains = evmChainOptions().map((row) => ({
  id: row.chainId,
  name: row.label,
  nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
  rpcUrls: { default: { http: [row.rpc] } },
}))

export const wagmiConfig = createConfig({
  chains: chains as [typeof chains[0], ...typeof chains],
  connectors: [injected()],
  transports: Object.fromEntries(
    evmChainOptions().map((row) => [row.chainId, http(row.rpc)])
  ),
})

const queryClient = new QueryClient()

export function EvmProviders({ children }: { children: ReactNode }) {
  return (
    <WagmiProvider config={wagmiConfig}>
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    </WagmiProvider>
  )
}

export function EvmWalletControls() {
  const { address, isConnected } = useAccount()
  const { connect, connectors, isPending } = useConnect()
  const { disconnect } = useDisconnect()

  if (isConnected && address) {
    return (
      <div className="wallet-actions">
        <span className="mono">{address}</span>
        <button type="button" className="secondary" onClick={() => disconnect()}>
          Disconnect
        </button>
      </div>
    )
  }

  return (
    <button
      type="button"
      className="primary"
      disabled={isPending}
      onClick={() => connect({ connector: connectors[0] })}
    >
      Connect EVM wallet
    </button>
  )
}

export { useAccount, useWalletClient }
