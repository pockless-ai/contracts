import assert from "node:assert/strict"
import test from "node:test"

import { decodeFunctionData, getCreate2Address, hashTypedData, keccak256 } from "viem"

import {
  EVM_NATIVE_TOKEN,
  GasFundingMode,
  SESSION_SPEND_VERSION,
  ZEROX_NATIVE_TOKEN,
  encodeCreditRelayAsset,
  encodeCreditUsdcReturn,
  encodeExecuteRelayDeposit,
  encodeExecuteRemoteRelaySell,
  encodeExecuteSwapWithFeesV2,
  encodeExecuteWalletRelaySwap,
  encodeRecoverVaultSurplus,
  encodeReleaseRelayDeposit,
  encodeRestoreRemoteRelayAsset,
  encodeSetPlatformRelayer,
  hashRelayCalldata,
  creditRelayAssetIntentDomain,
  creditRelayAssetIntentTypes,
  creditUsdcReturnIntentDomain,
  creditUsdcReturnIntentTypes,
  normalizeRelayOrderId,
  relayDepositIntentDomain,
  relayDepositIntentTypes,
  releaseRelayDepositIntentDomain,
  releaseRelayDepositIntentTypes,
  remoteRelaySellIntentDomain,
  remoteRelaySellIntentTypes,
  restoreRemoteRelayAssetIntentDomain,
  restoreRemoteRelayAssetIntentTypes,
  sessionSpend7702Abi,
  sessionSpendDomain,
  strategyVaultAddress,
  strategyVaultSalt,
  swapBundleIntentV2Domain,
  swapBundleIntentV2Types,
  walletRelaySwapIntentDomain,
  walletRelaySwapIntentTypes,
  type CreditRelayAssetIntentMessage,
  type CreditUsdcReturnIntentMessage,
  type RelayDepositIntentMessage,
  type RemoteRelaySellIntentMessage,
  type ReleaseRelayDepositIntentMessage,
  type RestoreRemoteRelayAssetIntentMessage,
  type SwapBundleIntentV2Message,
  type WalletRelaySwapIntentMessage,
} from "../../src/evm/session-spend-7702"

const strategyId = `0x${"11".repeat(32)}` as const
const sessionKey = `0x${"22".repeat(20)}` as const
const wallet = `0x${"33".repeat(20)}` as const
const usdc = `0x${"44".repeat(20)}` as const
const token = `0x${"55".repeat(20)}` as const
const recipient = `0x${"66".repeat(20)}` as const
const refundVault = `0x${"77".repeat(20)}` as const
const calldata = "0x2213bc0b00" as const
const calldataHash = keccak256(calldata)
const signature = `0x${"88".repeat(65)}` as const
const relayOrderId = `0x${"99".repeat(32)}` as const

function v2Intent(
  overrides: Partial<SwapBundleIntentV2Message>
): SwapBundleIntentV2Message {
  return {
    strategyId,
    sessionKey,
    nonce: 0n,
    deadline: 2_000_000_000n,
    sellToken: usdc,
    buyToken: token,
    strategySellAmount: 100_000_000n,
    minStrategyBuyAmount: 1n,
    strategyRouterCalldataHash: calldataHash,
    platformFeeUsdc: 0n,
    feeRecipient: EVM_NATIVE_TOKEN,
    gasFundingMode: GasFundingMode.CREDIT_ONLY,
    gasTopUpUsdc: 0n,
    gasTopUpNative: 0n,
    gasRecipient: EVM_NATIVE_TOKEN,
    gasRouterCalldataHash: `0x${"00".repeat(32)}`,
    ...overrides,
  }
}

test("all EIP-712 domains use version 1", () => {
  assert.equal(SESSION_SPEND_VERSION, "1")
  const domainInput = { chainId: 1, verifyingContract: wallet }
  for (const domain of [
    sessionSpendDomain(domainInput),
    swapBundleIntentV2Domain(domainInput),
    relayDepositIntentDomain(domainInput),
    creditRelayAssetIntentDomain(domainInput),
    remoteRelaySellIntentDomain(domainInput),
    creditUsdcReturnIntentDomain(domainInput),
    releaseRelayDepositIntentDomain(domainInput),
    restoreRemoteRelayAssetIntentDomain(domainInput),
    walletRelaySwapIntentDomain(domainInput),
  ]) {
    assert.equal(domain.version, "1")
  }
})

test("V2 typed data and ABI pin every gas funding mode", () => {
  const intents = [
    v2Intent({ gasFundingMode: GasFundingMode.CREDIT_ONLY }),
    v2Intent({
      gasFundingMode: GasFundingMode.SEPARATE_TOPUP,
      gasTopUpUsdc: 500_000n,
      gasTopUpNative: 500_000_000_000_000n,
      gasRecipient: recipient,
      gasRouterCalldataHash: calldataHash,
    }),
    v2Intent({
      buyToken: EVM_NATIVE_TOKEN,
      gasFundingMode: GasFundingMode.NATIVE_OUTPUT,
      gasTopUpUsdc: 500_000n,
      gasTopUpNative: 500_000_000_000_000n,
      gasRecipient: recipient,
    }),
  ]

  const hashes = new Set<string>()
  for (const intent of intents) {
    hashes.add(
      hashTypedData({
        domain: swapBundleIntentV2Domain({
          chainId: 1,
          verifyingContract: wallet,
        }),
        types: swapBundleIntentV2Types(),
        primaryType: "SwapBundleIntentV2",
        message: intent,
      })
    )
    const encoded = encodeExecuteSwapWithFeesV2({
      intent,
      strategyRouterCalldata: calldata,
      gasRouterCalldata:
        intent.gasFundingMode === GasFundingMode.SEPARATE_TOPUP
          ? calldata
          : "0x",
      sessionSignature: signature,
    })
    const decoded = decodeFunctionData({
      abi: sessionSpend7702Abi,
      data: encoded,
    })
    assert.equal(decoded.functionName, "executeSwapWithFeesV2")
    assert.deepEqual(decoded.args?.[0], intent)
  }
  assert.equal(hashes.size, 3)
})

test("relay deposit and credit return encode expanded intent fields", () => {
  const relayTarget = `0x${"aa".repeat(20)}` as const
  const relayCalldata = "0x1234" as const
  const relayValue = 0n
  const relayHash = hashRelayCalldata({
    relayTarget,
    relayValue,
    relayCalldata,
  })
  const relayIntent: RelayDepositIntentMessage = {
    strategyId,
    sessionKey,
    nonce: 0n,
    deadline: 2_000_000_000n,
    relayOrderId,
    fundingChainId: 1n,
    originToken: usdc,
    originTokenDecimals: 6,
    destToken: token,
    destTokenDecimals: 18,
    originAmount: 100_000_000n,
    destChainId: 42161n,
    minDestAmount: 1n,
    destRecipient: recipient,
    refundVault,
    relayCalldataHash: relayHash,
    platformFeeUsdc: 500_000n,
    feeRecipient: recipient,
  }
  const creditIntent: CreditUsdcReturnIntentMessage = {
    strategyId,
    sessionKey,
    nonce: 1n,
    deadline: 2_000_000_000n,
    relayOrderId,
    fundingChainId: 42161n,
    usdcReceived: 95_000_000n,
    destQuantityReleased: 1_000_000n,
    destCostReleasedUsdc: 100_000_000n,
    platformFeeUsdc: 500_000n,
    feeRecipient: recipient,
  }

  assert.equal(
    hashTypedData({
      domain: relayDepositIntentDomain({
        chainId: 1,
        verifyingContract: wallet,
      }),
      types: relayDepositIntentTypes(),
      primaryType: "RelayDepositIntent",
      message: relayIntent,
    }).length,
    66
  )
  assert.equal(
    hashTypedData({
      domain: creditUsdcReturnIntentDomain({
        chainId: 1,
        verifyingContract: wallet,
      }),
      types: creditUsdcReturnIntentTypes(),
      primaryType: "CreditUsdcReturnIntent",
      message: creditIntent,
    }).length,
    66
  )

  const relayEncoded = encodeExecuteRelayDeposit({
    intent: relayIntent,
    relayTarget,
    relayCalldata,
    relayValue,
    sessionSignature: signature,
  })
  const relayDecoded = decodeFunctionData({
    abi: sessionSpend7702Abi,
    data: relayEncoded,
  })
  assert.equal(relayDecoded.functionName, "executeRelayDeposit")
  assert.deepEqual(relayDecoded.args?.[0], relayIntent)

  const creditEncoded = encodeCreditUsdcReturn({
    intent: creditIntent,
    sessionSignature: signature,
  })
  const creditDecoded = decodeFunctionData({
    abi: sessionSpend7702Abi,
    data: creditEncoded,
  })
  assert.equal(creditDecoded.functionName, "creditUsdcReturn")
  assert.deepEqual(creditDecoded.args?.[0], creditIntent)
})

test("relay lifecycle encoders match ABI", () => {
  const creditAssetIntent: CreditRelayAssetIntentMessage = {
    strategyId,
    sessionKey,
    nonce: 2n,
    deadline: 2_000_000_000n,
    relayOrderId,
    token,
    fundingChainId: 42161n,
    creditQuantity: 1_000_000n,
    costUsdc: 50_000_000n,
  }
  const remoteSellIntent: RemoteRelaySellIntentMessage = {
    strategyId,
    sessionKey,
    nonce: 3n,
    deadline: 2_000_000_000n,
    relayOrderId,
    token,
    fundingChainId: 42161n,
    sellQuantity: 1_000_000n,
    minReturnUsdc: 90_000_000n,
    relayCalldataHash: calldataHash,
  }
  const releaseIntent: ReleaseRelayDepositIntentMessage = {
    strategyId,
    sessionKey,
    nonce: 4n,
    deadline: 2_000_000_000n,
    relayOrderId,
    token: usdc,
    fundingChainId: 1n,
    refundQuantity: 100_000_000n,
    refundCostUsdc: 100_000_000n,
  }
  const restoreIntent: RestoreRemoteRelayAssetIntentMessage = {
    strategyId,
    sessionKey,
    nonce: 5n,
    deadline: 2_000_000_000n,
    relayOrderId,
    token,
    fundingChainId: 42161n,
    restoreQuantity: 1_000_000n,
    restoreCostUsdc: 50_000_000n,
  }

  for (const [encoded, name] of [
    [encodeCreditRelayAsset({ intent: creditAssetIntent, sessionSignature: signature }), "creditRelayAsset"],
    [
      encodeExecuteRemoteRelaySell({
        intent: remoteSellIntent,
        relayTarget: recipient,
        relayCalldata: calldata,
        relayValue: 0n,
        sessionSignature: signature,
      }),
      "executeRemoteRelaySell",
    ],
    [encodeReleaseRelayDeposit({ intent: releaseIntent, sessionSignature: signature }), "releaseRelayDeposit"],
    [encodeRestoreRemoteRelayAsset({ intent: restoreIntent, sessionSignature: signature }), "restoreRemoteRelayAsset"],
    [encodeSetPlatformRelayer(recipient), "setPlatformRelayer"],
    [
      encodeRecoverVaultSurplus({
        strategyId,
        token: usdc,
        recipient,
        amount: 1n,
      }),
      "recoverVaultSurplus",
    ],
  ] as const) {
    const decoded = decodeFunctionData({
      abi: sessionSpend7702Abi,
      data: encoded,
    })
    assert.equal(decoded.functionName, name)
  }

  assert.equal(
    hashTypedData({
      domain: creditRelayAssetIntentDomain({
        chainId: 1,
        verifyingContract: wallet,
      }),
      types: creditRelayAssetIntentTypes(),
      primaryType: "CreditRelayAssetIntent",
      message: creditAssetIntent,
    }).length,
    66
  )
})

test("wallet relay swap uses relayOrderId", () => {
  const relayTarget = `0x${"bb".repeat(20)}` as const
  const relayCalldata = "0x1234" as const
  const relayValue = 0n
  const relayHash = hashRelayCalldata({
    relayTarget,
    relayValue,
    relayCalldata,
  })
  const intent: WalletRelaySwapIntentMessage = {
    nonce: 0n,
    deadline: 2_000_000_000n,
    sellToken: usdc,
    sellAmount: 100_000_000n,
    destRecipient: recipient,
    relayCalldataHash: relayHash,
    relayOrderId,
    platformFeeUsdc: 500_000n,
    feeRecipient: recipient,
  }

  assert.equal(
    hashTypedData({
      domain: walletRelaySwapIntentDomain({
        chainId: 1,
        verifyingContract: wallet,
      }),
      types: walletRelaySwapIntentTypes(),
      primaryType: "WalletRelaySwapIntent",
      message: intent,
    }).length,
    66
  )

  const encoded = encodeExecuteWalletRelaySwap({
    intent,
    relayTarget,
    relayCalldata,
    relayValue,
    ownerSignature: signature,
  })
  const decoded = decodeFunctionData({
    abi: sessionSpend7702Abi,
    data: encoded,
  })
  assert.equal(decoded.functionName, "executeWalletRelaySwap")
  assert.deepEqual(decoded.args?.[0], intent)
})

test("normalizeRelayOrderId validates 32-byte hex order ids", () => {
  assert.equal(normalizeRelayOrderId(relayOrderId), relayOrderId)
  assert.throws(() => normalizeRelayOrderId("not-an-order-id"))
})

test("strategyVaultAddress matches CREATE2 derivation", () => {
  const initCodeHash = `0x${"cc".repeat(32)}` as const
  assert.equal(
    strategyVaultAddress({
      sessionSpend: wallet,
      strategyId,
      initCodeHash,
    }),
    getCreate2Address({
      bytecodeHash: initCodeHash,
      from: wallet,
      salt: strategyVaultSalt(strategyId),
    })
  )
})

test("hashRelayCalldata hashes relay target, value, and calldata", () => {
  const relayTarget = `0x${"dd".repeat(20)}` as const
  const relayCalldata = "0xdeadbeef" as const
  const relayValue = 42n
  const hash = hashRelayCalldata({ relayTarget, relayValue, relayCalldata })
  assert.equal(hash.length, 66)
  assert.notEqual(
    hash,
    hashRelayCalldata({
      relayTarget,
      relayValue,
      relayCalldata: "0xbeef",
    })
  )
})

test("ZEROX native token constant is pinned", () => {
  assert.equal(ZEROX_NATIVE_TOKEN, "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE")
})
