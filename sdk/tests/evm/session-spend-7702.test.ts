import assert from "node:assert/strict"
import test from "node:test"

import { decodeFunctionData, hashTypedData, keccak256 } from "viem"

import {
  EVM_NATIVE_TOKEN,
  GasFundingMode,
  SESSION_SPEND_VERSION,
  SESSION_SPEND_V2_VERSION,
  ZEROX_NATIVE_TOKEN,
  encodeExecuteSwap,
  encodeExecuteSwapWithFeesV2,
  sessionSpend7702Abi,
  swapBundleIntentV2Domain,
  swapBundleIntentV2Types,
  swapIntentDomain,
  type SwapBundleIntentV2Message,
  type SwapIntentMessage,
} from "../../src/evm/session-spend-7702"

const strategyId = `0x${"11".repeat(32)}` as const
const sessionKey = `0x${"22".repeat(20)}` as const
const wallet = `0x${"33".repeat(20)}` as const
const usdc = `0x${"44".repeat(20)}` as const
const token = `0x${"55".repeat(20)}` as const
const recipient = `0x${"66".repeat(20)}` as const
const calldata = "0x2213bc0b00" as const
const calldataHash = keccak256(calldata)
const signature = `0x${"77".repeat(65)}` as const

function v1Intent(overrides: Partial<SwapIntentMessage>): SwapIntentMessage {
  return {
    strategyId,
    sessionKey,
    nonce: 0n,
    deadline: 2_000_000_000n,
    sellToken: usdc,
    buyToken: token,
    maxSellAmount: 100_000_000n,
    minBuyAmount: 1n,
    routerCalldataHash: calldataHash,
    ...overrides,
  }
}

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

test("V1 ABI encodes native buys and sells with address zero intents", () => {
  assert.equal(ZEROX_NATIVE_TOKEN, "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE")
  for (const intent of [
    v1Intent({ buyToken: EVM_NATIVE_TOKEN }),
    v1Intent({ sellToken: EVM_NATIVE_TOKEN, buyToken: usdc }),
  ]) {
    const encoded = encodeExecuteSwap({
      intent,
      routerCalldata: calldata,
      sessionSignature: signature,
    })
    const decoded = decodeFunctionData({
      abi: sessionSpend7702Abi,
      data: encoded,
    })
    assert.equal(decoded.functionName, "executeSwap")
    assert.deepEqual(decoded.args?.[0], intent)
  }
})

test("V1 and V2 use distinct EIP-712 domain versions", () => {
  assert.equal(SESSION_SPEND_VERSION, "1")
  assert.equal(SESSION_SPEND_V2_VERSION, "2")
  assert.equal(
    swapIntentDomain({ chainId: 1, verifyingContract: wallet }).version,
    "1"
  )
  assert.equal(
    swapBundleIntentV2Domain({ chainId: 1, verifyingContract: wallet }).version,
    "2"
  )
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
