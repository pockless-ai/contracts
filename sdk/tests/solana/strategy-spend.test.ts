import assert from "node:assert/strict"
import test from "node:test"
import { PublicKey } from "@solana/web3.js"

import {
  encodeExecuteSwapWithFees,
  encodeExecuteSwapWithFeesV2,
} from "../../src/solana/strategy-spend"

const treasury = new PublicKey("11111111111111111111111111111111")
const recipient = new PublicKey("BPFLoader1111111111111111111111111111111111")

test("zero gas reimbursement encodes an empty gas vector", () => {
  const data = encodeExecuteSwapWithFees({
    isBuy: true,
    usdcAmount: 1n,
    tokenAmount: 1n,
    platformFeeUsdc: 0n,
    gasReimburseUsdc: 0n,
    minNativeOut: 0n,
    treasury,
    jupiterData: Buffer.from([1]),
    gasJupiterData: Buffer.alloc(0),
    gasJupiterAccountCount: 0,
  })

  assert.equal(data.readUInt32LE(79), 0)
})

test("zero gas reimbursement rejects gas route fields", () => {
  assert.throws(() =>
    encodeExecuteSwapWithFees({
      isBuy: true,
      usdcAmount: 1n,
      tokenAmount: 1n,
      platformFeeUsdc: 0n,
      gasReimburseUsdc: 0n,
      minNativeOut: 1n,
      treasury,
      jupiterData: Buffer.from([1]),
      gasJupiterData: Buffer.from([1]),
      gasJupiterAccountCount: 1,
    })
  )
})

test("V2 native output uses the appended variant and exact Borsh layout", () => {
  const data = encodeExecuteSwapWithFeesV2({
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
    gasJupiterAccountCount: 0,
  })

  assert.equal(data[0], 9)
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

test("V2 separate mode prefixes the gas account count", () => {
  const data = encodeExecuteSwapWithFeesV2({
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
    gasJupiterAccountCount: 8,
  })

  assert.equal(data[26], 2)
  assert.equal(data.readUInt32LE(107), 1)
  assert.equal(data.readUInt32LE(112), 3)
  assert.deepEqual(data.subarray(116), Buffer.from([8, 6, 7]))
})

test("V2 none and credit-only reject on-chain top-up fields", () => {
  const base = {
    isBuy: true,
    usdcAmount: 1n,
    tokenAmount: 1n,
    platformFeeUsdc: 0n,
    treasury,
    gasRecipient: recipient,
    jupiterData: Buffer.from([1]),
    gasJupiterData: Buffer.alloc(0),
    gasJupiterAccountCount: 0,
  }

  assert.throws(() =>
    encodeExecuteSwapWithFeesV2({
      ...base,
      gasMode: "none",
      gasTopUpUsdc: 1n,
      nativeAmount: 0n,
    })
  )
  assert.throws(() =>
    encodeExecuteSwapWithFeesV2({
      ...base,
      gasMode: "credit_only",
      gasTopUpUsdc: 1n,
      nativeAmount: 1n,
    })
  )
})

test("V2 native output rejects sells and separate rejects missing routes", () => {
  const base = {
    usdcAmount: 1n,
    tokenAmount: 1n,
    platformFeeUsdc: 0n,
    gasTopUpUsdc: 1n,
    nativeAmount: 1n,
    treasury,
    gasRecipient: recipient,
    jupiterData: Buffer.from([1]),
    gasJupiterData: Buffer.alloc(0),
    gasJupiterAccountCount: 0,
  }

  assert.throws(() =>
    encodeExecuteSwapWithFeesV2({
      ...base,
      isBuy: false,
      gasMode: "native_output",
    })
  )
  assert.throws(() =>
    encodeExecuteSwapWithFeesV2({
      ...base,
      isBuy: true,
      gasMode: "separate",
    })
  )
})
