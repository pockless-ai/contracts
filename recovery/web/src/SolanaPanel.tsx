import { PublicKey } from "@solana/web3.js"
import { useState } from "react"
import {
  buildCloseInstruction,
  buildRevokeInstruction,
  buildWithdrawInstruction,
  listSolanaStrategies,
  sendSolanaWithWallet,
} from "../../lib/solana"
import { formatUsdc, parseStrategyId, type SolanaStrategy } from "../../lib/types"
import { solanaClusterOptions } from "./deployments"
import { useSolana } from "./solana-wallet"

export function SolanaPanel() {
  const clusters = solanaClusterOptions()
  const [cluster, setCluster] = useState(clusters[0]?.cluster ?? "mainnet-beta")
  const [strategies, setStrategies] = useState<SolanaStrategy[]>([])
  const [status, setStatus] = useState<string | null>(null)
  const [withdrawMint, setWithdrawMint] = useState("")
  const [withdrawAmount, setWithdrawAmount] = useState("")
  const [selectedStrategyId, setSelectedStrategyId] = useState("")
  const { wallet, connection } = useSolana()

  const selected = clusters.find((row) => row.cluster === cluster) ?? clusters[0]

  async function refresh() {
    if (!wallet.publicKey || !selected) return
    setStatus("Loading Solana strategies…")
    try {
      const rows = await listSolanaStrategies({
        connection,
        programId: new PublicKey(selected.programId),
        owner: wallet.publicKey,
      })
      setStrategies(rows)
      setStatus(rows.length ? null : "No strategies found for this owner.")
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to load strategies")
    }
  }

  async function revoke(strategy: SolanaStrategy) {
    if (!wallet.publicKey || !selected || !wallet.signTransaction) return
    setStatus("Submitting revoke…")
    try {
      const programId = new PublicKey(selected.programId)
      const strategyId = parseStrategyId(strategy.strategyId)
      const signature = await sendSolanaWithWallet({
        connection,
        payer: wallet.publicKey,
        signTransaction: wallet.signTransaction.bind(wallet),
        instructions: [
          buildRevokeInstruction({
            programId,
            owner: wallet.publicKey,
            strategyId,
          }),
        ],
      })
      setStatus(`Revoke confirmed: ${signature}`)
      await refresh()
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Revoke failed")
    }
  }

  async function withdraw() {
    if (!wallet.publicKey || !selected || !selectedStrategyId || !withdrawMint) return
    if (!wallet.signTransaction) {
      setStatus("Wallet cannot sign transactions")
      return
    }
    setStatus("Submitting withdraw…")
    try {
      const programId = new PublicKey(selected.programId)
      const strategyId = parseStrategyId(selectedStrategyId)
      const signature = await sendSolanaWithWallet({
        connection,
        payer: wallet.publicKey,
        signTransaction: wallet.signTransaction.bind(wallet),
        instructions: [
          buildWithdrawInstruction({
            programId,
            owner: wallet.publicKey,
            strategyId,
            mint: new PublicKey(withdrawMint),
            amount: BigInt(withdrawAmount || "0"),
          }),
        ],
      })
      setStatus(`Withdraw confirmed: ${signature}`)
      await refresh()
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Withdraw failed")
    }
  }

  async function close() {
    if (!wallet.publicKey || !selected || !selectedStrategyId) return
    if (!wallet.signTransaction) {
      setStatus("Wallet cannot sign transactions")
      return
    }
    setStatus("Submitting close…")
    try {
      const programId = new PublicKey(selected.programId)
      const strategyId = parseStrategyId(selectedStrategyId)
      const signature = await sendSolanaWithWallet({
        connection,
        payer: wallet.publicKey,
        signTransaction: wallet.signTransaction.bind(wallet),
        instructions: [
          buildCloseInstruction({
            programId,
            owner: wallet.publicKey,
            strategyId,
          }),
        ],
      })
      setStatus(`Close confirmed: ${signature}`)
      await refresh()
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Close failed")
    }
  }

  return (
    <section className="panel">
      <h2>Solana vaults</h2>
      <div className="row">
        <label>
          Cluster
          <select value={cluster} onChange={(event) => setCluster(event.target.value)}>
            {clusters.map((row) => (
              <option key={row.cluster} value={row.cluster}>
                {row.label}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className="primary"
          disabled={!wallet.publicKey}
          onClick={refresh}
        >
          Refresh
        </button>
      </div>

      {!wallet.publicKey ? (
        <p className="status">Connect a Solana wallet to list strategy vaults.</p>
      ) : null}

      {strategies.length > 0 ? (
        <table>
          <thead>
            <tr>
              <th>Strategy</th>
              <th>Session</th>
              <th>Limit / deployed</th>
              <th>Status</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {strategies.map((strategy) => (
              <tr key={strategy.pubkey}>
                <td className="mono">{strategy.strategyId}</td>
                <td className="mono">{strategy.session}</td>
                <td>
                  {formatUsdc(strategy.limitUsdc)} / {formatUsdc(strategy.deployedUsdc)}
                </td>
                <td>{strategy.revoked ? "revoked" : "active"}</td>
                <td>
                  <button
                    type="button"
                    className="secondary"
                    disabled={strategy.revoked}
                    onClick={() => revoke(strategy)}
                  >
                    Revoke
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}

      <h2 style={{ marginTop: 24 }}>Solana withdraw / close</h2>
      <div className="row">
        <label>
          Strategy id
          <input
            value={selectedStrategyId}
            onChange={(event) => setSelectedStrategyId(event.target.value)}
            placeholder="0x… (32 bytes)"
          />
        </label>
        <label>
          Mint
          <input
            value={withdrawMint}
            onChange={(event) => setWithdrawMint(event.target.value)}
            placeholder={selected?.usdcMint ?? "mint"}
          />
        </label>
        <label>
          Amount (base units)
          <input
            value={withdrawAmount}
            onChange={(event) => setWithdrawAmount(event.target.value)}
            placeholder="1000000"
          />
        </label>
      </div>
      <div className="row">
        <button type="button" className="primary" disabled={!wallet.publicKey} onClick={withdraw}>
          Withdraw
        </button>
        <button type="button" className="secondary" disabled={!wallet.publicKey} onClick={close}>
          Close strategy
        </button>
      </div>

      {status ? <p className="status">{status}</p> : null}
    </section>
  )
}
