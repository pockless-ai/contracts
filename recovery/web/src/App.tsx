import { useState } from "react"
import { EvmPanel } from "./EvmPanel"
import { EvmProviders, EvmWalletControls } from "./evm-wallet"
import { SolanaPanel } from "./SolanaPanel"
import { defaultSolanaRpc, SolanaProviders, SolanaWalletControls } from "./solana-wallet"

type Tab = "evm" | "solana"

function AppShell({
  solanaRpc,
  onSolanaRpcChange,
}: {
  solanaRpc: string
  onSolanaRpcChange: (value: string) => void
}) {
  const [tab, setTab] = useState<Tab>("evm")

  return (
    <div className="app">
      <header>
        <div>
          <h1>Owner recovery</h1>
          <p className="subtitle">
            Revoke sessions, withdraw vault assets, and close strategies with only your wallet
            and a public RPC.{" "}
            <a href="https://github.com/pockless-ai/contracts/blob/main/docs/recovery.md">
              Documentation
            </a>
          </p>
        </div>
        <div className="wallet-actions">
          {tab === "evm" ? <EvmWalletControls /> : <SolanaWalletControls />}
        </div>
      </header>

      <div className="tabs">
        <button
          type="button"
          className={tab === "evm" ? "active" : ""}
          onClick={() => setTab("evm")}
        >
          EVM
        </button>
        <button
          type="button"
          className={tab === "solana" ? "active" : ""}
          onClick={() => setTab("solana")}
        >
          Solana
        </button>
      </div>

      {tab === "evm" ? (
        <EvmPanel />
      ) : (
        <>
          <div className="row" style={{ marginBottom: 12 }}>
            <label>
              RPC URL
              <input
                value={solanaRpc}
                onChange={(event) => onSolanaRpcChange(event.target.value)}
              />
            </label>
          </div>
          <SolanaPanel />
        </>
      )}
    </div>
  )
}

export function App() {
  const [solanaRpc, setSolanaRpc] = useState(defaultSolanaRpc())

  return (
    <EvmProviders>
      <SolanaProviders rpc={solanaRpc}>
        <AppShell solanaRpc={solanaRpc} onSolanaRpcChange={setSolanaRpc} />
      </SolanaProviders>
    </EvmProviders>
  )
}
