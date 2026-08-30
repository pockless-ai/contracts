import assert from "node:assert/strict"
import test from "node:test"
import { PublicKey } from "@solana/web3.js"

import {
  encodeCloseStrategy,
  encodeCreditRelayAsset,
  encodeCreditUsdcReturn,
  encodeExecuteRelayDeposit,
  encodeExecuteRemoteRelaySell,
  encodeExecuteRelayGasTopUp,
  encodeReleaseRelayGasTopUp,
  encodeExecuteSwapWithFees,
  encodeInitWallet,
  encodeReleaseRelayDeposit,
  encodeRestoreRemoteRelayAsset,
  encodeWithdrawAsset,
  nativeVaultPda,
  relayReceiptPda,
  remoteAggregatePda,
  remoteAssetPda,
  RelayAction,
} from "../../src/solana/strategy-spend"

const programId = new PublicKey("11111111111111111111111111111111")
const strategy = new PublicKey("BPFLoader1111111111111111111111111111111111")
const mint = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
const treasury = new PublicKey("11111111111111111111111111111112")
const recipient = new PublicKey("BPFLoader2111111111111111111111111111111111")
const relayOrderId = Buffer.alloc(32, 7)

test("instruction variant indices match the program enum", () => {
  assert.equal(encodeInitWallet()[0], 0)
  assert.equal(encodeWithdrawAsset(1n)[0], 5)
  assert.equal(encodeCloseStrategy()[0], 6)
  assert.equal(
    encodeExecuteSwapWithFees({
      isBuy: true,
      usdcAmount: 1n,
      tokenAmount: 1n,
      platformFeeUsdc: 0n,
      gasMode: "credit_only",
      gasTopUpUsdc: 0n,
      nativeAmount: 0n,
      treasury,
      gasRecipient: recipient,
      jupiterData: Buffer.from([1]),
      gasJupiterData: Buffer.alloc(0),
    })[0],
    7
  )
  assert.equal(
    encodeExecuteRelayDeposit({
      relayOrderId,
      fundingChainId: 1n,
      amount: 2n,
      minDestAmount: 3n,
      lockedCostUsdc: 4n,
      platformFeeUsdc: 5n,
      nonce: 6n,
      deadline: 7n,
      relayIxData: Buffer.from([8]),
    })[0],
    8
  )
  assert.equal(
    encodeCreditRelayAsset({
      relayOrderId,
      fundingChainId: 1n,
      creditQuantity: 1n,
      costUsdc: 2n,
      minCreditQty: 3n,
      maxCreditQty: 4n,
      nonce: 5n,
      deadline: 6n,
    })[0],
    9
  )
  assert.equal(
    encodeExecuteRemoteRelaySell({
      relayOrderId,
      fundingChainId: 1n,
      sellQuantity: 1n,
      minReturnUsdc: 2n,
      nonce: 3n,
      deadline: 4n,
      relayIxData: Buffer.from([9]),
    })[0],
    10
  )
  assert.equal(
    encodeCreditUsdcReturn({
      relayOrderId,
      fundingChainId: 1n,
      grossReturnUsdc: 1n,
      quantityReleased: 2n,
      costReleasedUsdc: 3n,
      platformFeeUsdc: 4n,
      nonce: 5n,
      deadline: 6n,
    })[0],
    11
  )
  assert.equal(
    encodeReleaseRelayDeposit({
      relayOrderId,
      refundAmountUsdc: 1n,
      lockedCostUsdc: 2n,
      nonce: 3n,
      deadline: 4n,
    })[0],
    12
  )
  assert.equal(
    encodeRestoreRemoteRelayAsset({
      relayOrderId,
      fundingChainId: 1n,
      restoreQuantity: 1n,
      restoreCostUsdc: 2n,
      nonce: 3n,
      deadline: 4n,
    })[0],
    13
  )
  assert.equal(
    encodeExecuteRelayGasTopUp({
      relayOrderId,
      overheadUsdc: 1n,
      gasRecipient: recipient,
      nonce: 2n,
      deadline: 3n,
      relayIxData: Buffer.from([4]),
    })[0],
    14
  )
  assert.equal(
    encodeReleaseRelayGasTopUp({
      relayOrderId,
      refundUsdc: 1n,
      nonce: 2n,
      deadline: 3n,
    })[0],
    15
  )
})

test("ExecuteSwapWithFees uses Borsh vec layout at variant 7", () => {
  const data = encodeExecuteSwapWithFees({
    isBuy: true,
    usdcAmount: 100n,
    tokenAmount: 200n,
    platformFeeUsdc: 3n,
    gasMode: "native_output",
    gasTopUpUsdc: 4n,
    nativeAmount: 5n,
    treasury,
    gasRecipient: recipient,
    jupiterData: Buffer.from([6, 7]),
    gasJupiterData: Buffer.alloc(0),
  })

  assert.equal(data[0], 7)
  assert.equal(data[1], 1)
  assert.equal(data.readBigUInt64LE(2), 100n)
  assert.equal(data.readBigUInt64LE(10), 200n)
  assert.equal(data.readBigUInt64LE(18), 3n)
  assert.equal(data[26], 3)
  assert.equal(data.readBigUInt64LE(27), 4n)
  assert.equal(data.readBigUInt64LE(35), 5n)
  assert.deepEqual(data.subarray(43, 75), treasury.toBuffer())
  assert.deepEqual(data.subarray(75, 107), recipient.toBuffer())
  assert.equal(data.readUInt32LE(107), 2)
  assert.deepEqual(data.subarray(111, 113), Buffer.from([6, 7]))
  assert.equal(data.readUInt32LE(113), 0)
})

test("ExecuteSwapWithFees separate mode appends gas jupiter data", () => {
  const data = encodeExecuteSwapWithFees({
    isBuy: false,
    usdcAmount: 1n,
    tokenAmount: 2n,
    platformFeeUsdc: 0n,
    gasMode: "separate",
    gasTopUpUsdc: 3n,
    nativeAmount: 4n,
    treasury,
    gasRecipient: recipient,
    jupiterData: Buffer.from([5]),
    gasJupiterData: Buffer.from([6, 7]),
  })

  assert.equal(data[26], 2)
  assert.equal(data.readUInt32LE(107), 1)
  assert.equal(data.readUInt32LE(112), 2)
  assert.deepEqual(data.subarray(116), Buffer.from([6, 7]))
})

test("ExecuteSwapWithFees rejects invalid gas mode combinations", () => {
  const base = {
    isBuy: true,
    usdcAmount: 1n,
    tokenAmount: 1n,
    platformFeeUsdc: 0n,
    treasury,
    gasRecipient: recipient,
    jupiterData: Buffer.from([1]),
    gasJupiterData: Buffer.alloc(0),
  }

  assert.throws(() =>
    encodeExecuteSwapWithFees({
      ...base,
      gasMode: "none",
      gasTopUpUsdc: 1n,
      nativeAmount: 0n,
    })
  )
  assert.throws(() =>
    encodeExecuteSwapWithFees({
      ...base,
      gasMode: "native_output",
      gasTopUpUsdc: 1n,
      nativeAmount: 1n,
      isBuy: false,
    })
  )
})

test("relay PDAs derive with program seeds", () => {
  const [receipt] = relayReceiptPda(
    programId,
    strategy,
    relayOrderId,
    RelayAction.Deposit
  )
  const [remoteAsset] = remoteAssetPda(programId, strategy, mint, 42161n)
  const [aggregate] = remoteAggregatePda(programId, strategy, mint)
  const [nativeVault] = nativeVaultPda(programId, strategy)

  assert.notEqual(receipt.toBase58(), strategy.toBase58())
  assert.notEqual(remoteAsset.toBase58(), aggregate.toBase58())
  assert.notEqual(nativeVault.toBase58(), strategy.toBase58())
})

test("ExecuteRelayDeposit encodes relay ix data as Borsh vec", () => {
  const data = encodeExecuteRelayDeposit({
    relayOrderId,
    fundingChainId: 8453n,
    amount: 100n,
    minDestAmount: 90n,
    lockedCostUsdc: 100n,
    platformFeeUsdc: 1n,
    nonce: 2n,
    deadline: 3n,
    relayIxData: Buffer.from([9, 10]),
  })

  assert.equal(data.readBigUInt64LE(33), 8453n)
  assert.equal(data.readUInt32LE(89), 2)
  assert.deepEqual(data.subarray(93), Buffer.from([9, 10]))
})
