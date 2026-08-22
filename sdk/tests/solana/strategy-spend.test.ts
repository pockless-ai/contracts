import assert from "node:assert/strict"
import test from "node:test"
import { PublicKey } from "@solana/web3.js"

import { encodeExecuteSwapWithFees } from "../../src/solana/strategy-spend"

const treasury = new PublicKey("11111111111111111111111111111111")

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
