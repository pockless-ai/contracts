import { WalletAdapterNetwork } from "@solana/wallet-adapter-base"
import {
  ConnectionProvider,
  WalletProvider,
  useConnection,
  useWallet,
} from "@solana/wallet-adapter-react"
import { WalletModalProvider, WalletMultiButton } from "@solana/wallet-adapter-react-ui"
import { PhantomWalletAdapter, SolflareWalletAdapter } from "@solana/wallet-adapter-wallets"
import { useMemo, type ReactNode } from "react"
import { solanaClusterOptions } from "./deployments"

import "@solana/wallet-adapter-react-ui/styles.css"

export function SolanaProviders({
  children,
  rpc,
}: {
  children: ReactNode
  rpc: string
}) {
  const wallets = useMemo(
    () => [new PhantomWalletAdapter(), new SolflareWalletAdapter()],
    []
  )

  return (
    <ConnectionProvider endpoint={rpc}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>{children}</WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  )
}

export function SolanaWalletControls() {
  return <WalletMultiButton />
}

export function useSolana() {
  const wallet = useWallet()
  const connection = useConnection()
  return { wallet, connection: connection.connection }
}

export function defaultSolanaRpc() {
  return solanaClusterOptions()[0]?.rpc ?? "https://api.mainnet-beta.solana.com"
}

export { WalletAdapterNetwork }
