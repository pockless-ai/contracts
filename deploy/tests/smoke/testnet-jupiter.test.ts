import assert from "node:assert/strict"
import test from "node:test"

import { Keypair } from "@solana/web3.js"
import bs58 from "bs58"
import { configureTradeFees } from "@workspace/trade"
import { configureTradeLive } from "@workspace/trade/live"
import { prepareSolanaOwnerSwap } from "@workspace/trade/providers"

import { smokeEnabled } from "./lib/config"

/**
 * Jupiter has no devnet liquidity, so route composition is exercised against
 * mainnet quotes. Nothing here signs with a funded key or broadcasts — the
 * assertions are that a route exists and that the assembled transaction fits
 * inside one Solana packet, which is the failure mode that actually bites.
 */
const SOLANA_MAX_TRANSACTION_BYTES = 1232

const USDC = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
const BONK = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"
const JUP = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"
const WSOL = "So11111111111111111111111111111111111111112"
// Any mainnet account with a USDC token account; only read for account presence.
const PROBE_OWNER = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM"

const PAIRS = [
  { label: "BONK -> USDC", sell: BONK, buy: USDC, amount: 1_000_000_000_000n },
  { label: "BONK -> JUP", sell: BONK, buy: JUP, amount: 1_000_000_000_000n },
  { label: "JUP -> USDC", sell: JUP, buy: USDC, amount: 100_000_000n },
]

function configure() {
  configureTradeFees({
    feeBps: 50,
    feeCapUsdc: "10.000000",
    gasFloorUsdc: "1.000000",
  })
  configureTradeLive({
    RELAY_API_KEY: "",
    ZEROX_API_KEY: "",
    JUPITER_API_KEY: process.env.JUPITER_API_KEY?.trim() ?? "",
    JUPITER_RPS: process.env.JUPITER_API_KEY?.trim() ? 10 : 1,
    ALCHEMY_API_KEY: process.env.ALCHEMY_API_KEY?.trim() ?? "",
    ANKR_API_KEY: process.env.ANKR_API_KEY?.trim() ?? "",
    TRADE_SESSION_KEK: "",
    EVM_RELAYER_PRIVATE_KEY: "",
    SOLANA_RELAYER_SECRET_KEY: bs58.encode(Keypair.generate().secretKey),
    TREASURY_EVM_ADDRESS: "",
    TREASURY_SOLANA_PUBKEY: PROBE_OWNER,
    EVM_7702_IMPLEMENTATION: "",
    EVM_7702_ETHEREUM: "",
    EVM_7702_BASE: "",
    EVM_7702_ARBITRUM: "",
    EVM_7702_OPTIMISM: "",
    EVM_7702_POLYGON: "",
    EVM_7702_BNB: "",
    SOLANA_SPEND_PROGRAM_ID: "",
    GAS_FLOOR_USDC: "1.000000",
    GAS_SAFETY_MARGIN_BPS: 2000,
    SOLANA_COMPUTE_UNIT_LIMIT: 1_400_000,
    SOLANA_COMPUTE_UNIT_PRICE_MICROLAMPORTS: 0,
  })
}

test("Jupiter route composition smoke", { skip: !smokeEnabled() }, async (t) => {
  configure()

  if (!process.env.JUPITER_API_KEY?.trim()) {
    t.diagnostic(
      "JUPITER_API_KEY is unset; using lite-api.jup.ag. Set the key to use the paid host."
    )
  }

  for (const pair of PAIRS) {
    await t.test(`gasless ${pair.label} fits one Solana packet`, async () => {
      const result = await prepareSolanaOwnerSwap({
        owner: PROBE_OWNER,
        sellMint: pair.sell,
        buyMint: pair.buy,
        sellAmount: pair.amount,
        slippageBps: 100,
        gasless: true,
      })
      const bytes = Buffer.from(result.transaction, "base64").length
      assert.ok(
        bytes <= SOLANA_MAX_TRANSACTION_BYTES,
        `${pair.label} composed to ${bytes} bytes, over the ${SOLANA_MAX_TRANSACTION_BYTES} limit`
      )
      assert.ok(
        BigInt(result.buyAmount) > 0n,
        `${pair.label} produced no output amount`
      )
      assert.ok(
        Number(result.gasUsdc) > 0,
        `${pair.label} did not price a gas reimbursement`
      )
    })
  }

  await t.test("gasless swaps refuse to sell the fee asset", async () => {
    await assert.rejects(
      prepareSolanaOwnerSwap({
        owner: PROBE_OWNER,
        sellMint: WSOL,
        buyMint: USDC,
        sellAmount: 1_000_000_000n,
        slippageBps: 100,
        gasless: true,
      }),
      /cannot sell SOL/
    )
  })

  await t.test("gasless swaps refuse amounts below the gas floor", async () => {
    await assert.rejects(
      prepareSolanaOwnerSwap({
        owner: PROBE_OWNER,
        sellMint: JUP,
        buyMint: USDC,
        sellAmount: 10_000n,
        slippageBps: 100,
        gasless: true,
      }),
      /cannot cover the Solana network fee|no route/i
    )
  })

  await t.test("funded swaps build without a gas leg", async () => {
    const result = await prepareSolanaOwnerSwap({
      owner: PROBE_OWNER,
      sellMint: BONK,
      buyMint: USDC,
      sellAmount: 1_000_000_000_000n,
      slippageBps: 100,
      gasless: false,
    })
    assert.equal(result.gasUsdc, "0.000000")
    assert.ok(BigInt(result.buyAmount) > 0n)
  })
})
