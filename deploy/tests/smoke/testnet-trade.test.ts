import assert from "node:assert/strict"
import test from "node:test"
import {
  authorityPda,
  nativeVaultPda,
  relayPendingDepositPda,
  relayPendingGasTopUpPda,
  relayPendingSellPda,
  relayReceiptPda,
  remoteAggregatePda,
  remoteAssetPda,
  sessionSpend7702Abi,
  SolanaRelayAction,
  strategyPda,
  strategyVaultAddress,
  vaultPda,
  walletPda,
} from "@pockless/protocol-sdk"
import { Keypair, PublicKey } from "@solana/web3.js"
import {
  getAddress,
  keccak256,
  toBytes,
  toFunctionSelector,
  type Hex,
} from "viem"
import { assertEvmSignerAccess, loadSmokeConfig, smokeEnabled } from "./lib/config"
import {
  clearEvmDelegation,
  ensureEvmDelegation,
  evmClient,
  uniqueStrategyId,
} from "./lib/evm"
import {
  closeSolanaStrategy,
  ensureSolanaWallet,
  initSolanaStrategy,
  loadSolanaOwner,
  revokeSolanaStrategy,
  solanaConnection,
  uniqueSolanaStrategyId,
} from "./lib/solana"

const SOLANA_USDC_DEVNET_MINT = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"

function tradePathSelectors(): Array<{ signature: string; selector: Hex }> {
  return sessionSpend7702Abi
    .filter(
      (entry): entry is Extract<typeof entry, { type: "function" }> =>
        entry.type === "function"
    )
    .map((entry) => {
      const signature = `function ${entry.name}(${entry.inputs
        .map((input) => input.type)
        .join(",")})`
      return { signature, selector: toFunctionSelector(signature) }
    })
}

test("testnet trade path smoke", { skip: !smokeEnabled() }, async (t) => {
  const config = await loadSmokeConfig()

  await t.test(
    "Base Sepolia implements every SessionSpend7702 entrypoint the trade runtime encodes",
    async () => {
      const code = await evmClient(config.evm).getCode({
        address: config.evm.implementation,
      })
      assert.ok(code && code !== "0x", "implementation has no runtime code")

      const missing = tradePathSelectors().filter(
        ({ selector }) => !code.includes(selector.slice(2))
      )
      assert.deepEqual(
        missing.map((entry) => entry.signature),
        [],
        "deployed implementation is missing entrypoints the SDK encodes against"
      )
    }
  )

  await t.test(
    "Base Sepolia strategy vault derivation matches the deployed implementation",
    async () => {
      assertEvmSignerAccess(config.evm)
      const client = evmClient(config.evm)
      const strategyId = uniqueStrategyId("smoke-vault")

      const initCodeHash = await client.readContract({
        address: config.evm.implementation,
        abi: sessionSpend7702Abi,
        functionName: "strategyVaultInitCodeHash",
        args: [config.evm.owner],
      })
      const derived = strategyVaultAddress({
        sessionSpend: config.evm.owner,
        strategyId,
        initCodeHash,
      })

      // predictStrategyVault only resolves once the owner EOA carries the
      // delegation, which is also the state every live trade runs under.
      await ensureEvmDelegation(config.evm)
      try {
        const predicted = await client.readContract({
          address: config.evm.owner,
          abi: sessionSpend7702Abi,
          functionName: "predictStrategyVault",
          args: [strategyId],
        })
        assert.equal(getAddress(predicted), getAddress(derived))
      } finally {
        await clearEvmDelegation(config.evm)
      }
    }
  )

  await t.test(
    "Solana devnet strategy accounts match the addresses the trade runtime derives",
    async () => {
      const owner = await loadSolanaOwner(config.solana.ownerKeypairPath)
      const connection = solanaConnection(config.solana.rpc)
      const programId = new PublicKey(config.solana.programId)
      const strategyId = uniqueSolanaStrategyId("smoke-trade")
      const session = Keypair.generate()

      const wallet = await ensureSolanaWallet({
        connection,
        config: config.solana,
        owner,
      })
      const strategy = await initSolanaStrategy({
        connection,
        config: config.solana,
        owner,
        strategyId,
        session,
      })

      try {
        const [derivedWallet] = walletPda(programId, owner.publicKey)
        const [derivedStrategy] = strategyPda(
          programId,
          owner.publicKey,
          strategyId
        )
        assert.equal(derivedWallet.toBase58(), wallet.toBase58())
        assert.equal(derivedStrategy.toBase58(), strategy.toBase58())

        const strategyAccount = await connection.getAccountInfo(strategy)
        assert.equal(strategyAccount?.owner.toBase58(), programId.toBase58())

        // Every relay account the trade runtime passes is program-derived from
        // the strategy, so a seed change on redeploy shows up here rather than
        // as a failed relay settlement.
        const relayOrderId = Buffer.from(
          keccak256(toBytes(`smoke-relay-${Date.now()}`)).slice(2),
          "hex"
        )
        const mint = new PublicKey(SOLANA_USDC_DEVNET_MINT)
        const relayAccounts = [
          authorityPda(programId, owner.publicKey),
          vaultPda(programId, strategy),
          nativeVaultPda(programId, strategy),
          relayPendingDepositPda(programId, strategy, relayOrderId),
          relayPendingSellPda(programId, strategy, relayOrderId),
          relayPendingGasTopUpPda(programId, strategy, relayOrderId),
          relayReceiptPda(
            programId,
            strategy,
            relayOrderId,
            SolanaRelayAction.Deposit
          ),
          remoteAssetPda(programId, strategy, mint, 8453n),
          remoteAggregatePda(programId, strategy, mint),
        ]
        for (const [address] of relayAccounts) {
          assert.equal(
            PublicKey.isOnCurve(address.toBytes()),
            false,
            `${address.toBase58()} is not a program-derived address`
          )
        }
      } finally {
        await revokeSolanaStrategy({
          connection,
          config: config.solana,
          owner,
          strategyId,
        })
        await closeSolanaStrategy({
          connection,
          config: config.solana,
          owner,
          strategyId,
        })
      }
    }
  )
})
