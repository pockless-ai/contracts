import assert from "node:assert/strict"
import { Keypair } from "@solana/web3.js"
import { generatePrivateKey, privateKeyToAccount } from "viem/accounts"
import test from "node:test"
import { assertEvmSignerAccess, loadSmokeConfig, smokeEnabled } from "./lib/config"
import {
  assertEvmDeployment,
  cleanupEvmSession,
  clearEvmDelegation,
  grantEvmSession,
  isEvmDelegated,
  readEvmSession,
  revokeEvmSession,
  uniqueStrategyId,
} from "./lib/evm"
import {
  assertSolanaDeployment,
  closeSolanaStrategy,
  initSolanaStrategy,
  loadSolanaOwner,
  readSolanaStrategy,
  revokeSolanaStrategy,
  solanaConnection,
  uniqueSolanaStrategyId,
  ensureSolanaWallet,
} from "./lib/solana"

test("testnet contract smoke", { skip: !smokeEnabled() }, async (t) => {
  const config = await loadSmokeConfig()

  await t.test("Base Sepolia SessionSpend7702 lifecycle", async () => {
    assertEvmSignerAccess(config.evm)
    await assertEvmDeployment(config.evm)

    const strategyId = uniqueStrategyId("smoke-evm")
    const session = privateKeyToAccount(generatePrivateKey())
    const limitUsdc = 1_000_000n
    const expiresAt = BigInt(Math.floor(Date.now() / 1000) + 3600)
    let grantedSession:
      | { strategyId: typeof strategyId; sessionKey: typeof session.address }
      | undefined

    try {
      const grantTx = await grantEvmSession({
        config: config.evm,
        strategyId,
        sessionKey: session.address,
        limitUsdc,
        expiresAt,
      })
      grantedSession = { strategyId, sessionKey: session.address }
      assert.match(grantTx, /0x[0-9a-fA-F]{64}/)

      const sessionState = await readEvmSession({
        config: config.evm,
        strategyId,
        sessionKey: session.address,
      })
      assert.equal(sessionState.exists, true)
      assert.equal(sessionState.revoked, false)
      assert.equal(BigInt(sessionState.limitUsdc), limitUsdc)
      assert.equal(BigInt(sessionState.capacityUsdc), limitUsdc)
      assert.equal(BigInt(sessionState.deployedUsdc), 0n)
      assert.equal(BigInt(sessionState.nonce), 0n)
      assert.equal(BigInt(sessionState.expiresAt), expiresAt)

      const revokeTx = await revokeEvmSession({
        config: config.evm,
        strategyId,
        sessionKey: session.address,
      })
      assert.match(revokeTx, /0x[0-9a-fA-F]{64}/)

      const revoked = await readEvmSession({
        config: config.evm,
        strategyId,
        sessionKey: session.address,
      })
      assert.equal(revoked.revoked, true)
    } finally {
      if (grantedSession) {
        await cleanupEvmSession({
          config: config.evm,
          ...grantedSession,
        })
      }
      await clearEvmDelegation(config.evm)
      assert.equal(await isEvmDelegated(config.evm), false)
    }
  })

  await t.test("Solana devnet strategy-spend lifecycle", async () => {
    await assertSolanaDeployment(config.solana)

    const owner = await loadSolanaOwner(config.solana.ownerKeypairPath)
    const connection = solanaConnection(config.solana.rpc)
    const strategyId = uniqueSolanaStrategyId("smoke-solana")
    const session = Keypair.generate()
    const limitUsdc = 1_000_000n
    const expiresAt = BigInt(Math.floor(Date.now() / 1000) + 3600)

    await ensureSolanaWallet({ connection, config: config.solana, owner })

    const strategy = await initSolanaStrategy({
      connection,
      config: config.solana,
      owner,
      strategyId,
      session,
      limitUsdc,
      expiresAt,
    })

    const created = await readSolanaStrategy({ connection, strategy })
    assert.equal(created.revoked, false)
    assert.equal(created.limitUsdc, limitUsdc)
    assert.equal(created.capacityUsdc, limitUsdc)
    assert.equal(created.deployedUsdc, 0n)
    assert.equal(created.nonce, 0n)
    assert.equal(created.expiresAt, expiresAt)
    assert.equal(created.session, session.publicKey.toBase58())

    const revokeSignature = await revokeSolanaStrategy({
      connection,
      config: config.solana,
      owner,
      strategyId,
    })
    assert.match(revokeSignature, /^[1-9A-HJ-NP-Za-km-z]{87,88}$/)

    const revoked = await readSolanaStrategy({ connection, strategy })
    assert.equal(revoked.revoked, true)

    const closeSignature = await closeSolanaStrategy({
      connection,
      config: config.solana,
      owner,
      strategyId,
    })
    assert.match(closeSignature, /^[1-9A-HJ-NP-Za-km-z]{87,88}$/)

    const closed = await connection.getAccountInfo(strategy)
    assert.equal(closed, null)
  })
})
