import { useState } from "react"
import { type Address } from "viem"
import {
  createEvmClient,
  listEvmSessions,
  revokeEvmSession,
  withdrawEvmToken,
} from "../../lib/evm"
import { formatUsdc, type EvmSession } from "../../lib/types"
import { evmChainOptions } from "./deployments"
import { useAccount, useWalletClient } from "./evm-wallet"

export function EvmPanel() {
  const chains = evmChainOptions()
  const [chainId, setChainId] = useState(chains[0]?.chainId ?? 1)
  const [sessions, setSessions] = useState<EvmSession[]>([])
  const [status, setStatus] = useState<string | null>(null)
  const [withdrawToken, setWithdrawToken] = useState("")
  const [withdrawAmount, setWithdrawAmount] = useState("")
  const { address } = useAccount()
  const { data: walletClient } = useWalletClient()

  const selected = chains.find((row) => row.chainId === chainId) ?? chains[0]

  async function refresh() {
    if (!address || !selected) return
    setStatus("Loading EVM sessions…")
    try {
      const client = createEvmClient(selected.rpc, selected.chainId)
      const rows = await listEvmSessions(client, address)
      setSessions(rows)
      setStatus(rows.length ? null : "No sessions found on this wallet.")
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to load sessions")
    }
  }

  async function revoke(session: EvmSession) {
    if (!address || !walletClient) return
    setStatus("Submitting revoke…")
    try {
      const hash = await revokeEvmSession({
        wallet: walletClient,
        owner: address,
        strategyId: session.strategyId,
        sessionKey: session.sessionKey,
      })
      setStatus(`Revoke submitted: ${hash}`)
      await refresh()
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Revoke failed")
    }
  }

  async function withdraw() {
    if (!address || !walletClient || !withdrawToken || !withdrawAmount) return
    setStatus("Submitting withdraw…")
    try {
      const hash = await withdrawEvmToken({
        wallet: walletClient,
        owner: address,
        token: withdrawToken as Address,
        to: address,
        amount: BigInt(withdrawAmount),
      })
      setStatus(`Withdraw submitted: ${hash}`)
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Withdraw failed")
    }
  }

  return (
    <section className="panel">
      <h2>EVM sessions</h2>
      <div className="row">
        <label>
          Chain
          <select
            value={chainId}
            onChange={(event) => setChainId(Number(event.target.value))}
          >
            {chains.map((row) => (
              <option key={row.chainId} value={row.chainId}>
                {row.label} ({row.chainId})
              </option>
            ))}
          </select>
        </label>
        <button type="button" className="primary" disabled={!address} onClick={refresh}>
          Refresh
        </button>
      </div>

      {!address ? (
        <p className="status">Connect an EVM wallet to list delegated sessions.</p>
      ) : null}

      {sessions.length > 0 ? (
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
            {sessions.map((session) => (
              <tr key={`${session.strategyId}-${session.sessionKey}`}>
                <td className="mono">{session.strategyId}</td>
                <td className="mono">{session.sessionKey}</td>
                <td>
                  {formatUsdc(session.limitUsdc)} / {formatUsdc(session.deployedUsdc)}
                </td>
                <td>{session.revoked ? "revoked" : "active"}</td>
                <td>
                  <button
                    type="button"
                    className="secondary"
                    disabled={session.revoked || !walletClient}
                    onClick={() => revoke(session)}
                  >
                    Revoke
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}

      <h2 style={{ marginTop: 24 }}>EVM withdraw</h2>
      <p className="status">
        Transfer ERC-20 tokens from your wallet. Revoke sessions first if funds were
        acquired through automated swaps.
      </p>
      <div className="row">
        <label>
          Token
          <input
            value={withdrawToken}
            onChange={(event) => setWithdrawToken(event.target.value)}
            placeholder="0x…"
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
        <button
          type="button"
          className="primary"
          disabled={!address || !walletClient}
          onClick={withdraw}
        >
          Withdraw
        </button>
      </div>

      {status ? <p className="status">{status}</p> : null}
    </section>
  )
}
