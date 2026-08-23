import {
  encodeFunctionData,
  keccak256,
  parseUnits,
  toBytes,
  type Address,
  type Hex,
} from "viem"

export const SESSION_SPEND_NAME = "PocklessSessionSpend7702"
export const SESSION_SPEND_VERSION = "1"
export const SESSION_SPEND_V2_VERSION = "2"
export const EVM_NATIVE_TOKEN = "0x0000000000000000000000000000000000000000"
export const ZEROX_NATIVE_TOKEN = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE"

export const GasFundingMode = {
  CREDIT_ONLY: 0,
  SEPARATE_TOPUP: 1,
  NATIVE_OUTPUT: 2,
} as const

export type GasFundingMode =
  (typeof GasFundingMode)[keyof typeof GasFundingMode]

export const sessionSpend7702Abi = [
  {
    type: "function",
    name: "grant",
    stateMutability: "nonpayable",
    inputs: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "limitUsdc", type: "uint256" },
      { name: "expiresAt", type: "uint256" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "setLimit",
    stateMutability: "nonpayable",
    inputs: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "newLimitUsdc", type: "uint256" },
      { name: "expiresAt", type: "uint256" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "rotateSession",
    stateMutability: "nonpayable",
    inputs: [
      { name: "strategyId", type: "bytes32" },
      { name: "oldKey", type: "address" },
      { name: "newKey", type: "address" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "revoke",
    stateMutability: "nonpayable",
    inputs: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "revokeWithSignature",
    stateMutability: "nonpayable",
    inputs: [
      {
        name: "intent",
        type: "tuple",
        components: [
          { name: "strategyId", type: "bytes32" },
          { name: "sessionKey", type: "address" },
          { name: "nonce", type: "uint256" },
          { name: "deadline", type: "uint256" },
        ],
      },
      { name: "sessionSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "executeSwap",
    stateMutability: "nonpayable",
    inputs: [
      {
        name: "intent",
        type: "tuple",
        components: [
          { name: "strategyId", type: "bytes32" },
          { name: "sessionKey", type: "address" },
          { name: "nonce", type: "uint256" },
          { name: "deadline", type: "uint256" },
          { name: "sellToken", type: "address" },
          { name: "buyToken", type: "address" },
          { name: "maxSellAmount", type: "uint256" },
          { name: "minBuyAmount", type: "uint256" },
          { name: "routerCalldataHash", type: "bytes32" },
        ],
      },
      { name: "routerCalldata", type: "bytes" },
      { name: "sessionSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "executeSwapWithFees",
    stateMutability: "nonpayable",
    inputs: [
      {
        name: "intent",
        type: "tuple",
        components: [
          { name: "strategyId", type: "bytes32" },
          { name: "sessionKey", type: "address" },
          { name: "nonce", type: "uint256" },
          { name: "deadline", type: "uint256" },
          { name: "sellToken", type: "address" },
          { name: "buyToken", type: "address" },
          { name: "maxSellAmount", type: "uint256" },
          { name: "minBuyAmount", type: "uint256" },
          { name: "routerCalldataHash", type: "bytes32" },
          { name: "platformFeeUsdc", type: "uint256" },
          { name: "feeRecipient", type: "address" },
          { name: "gasSellUsdc", type: "uint256" },
          { name: "minNativeOut", type: "uint256" },
          { name: "gasRecipient", type: "address" },
          { name: "gasRouterCalldataHash", type: "bytes32" },
        ],
      },
      { name: "strategyRouterCalldata", type: "bytes" },
      { name: "gasRouterCalldata", type: "bytes" },
      { name: "sessionSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "executeSwapWithFeesV2",
    stateMutability: "nonpayable",
    inputs: [
      {
        name: "intent",
        type: "tuple",
        components: [
          { name: "strategyId", type: "bytes32" },
          { name: "sessionKey", type: "address" },
          { name: "nonce", type: "uint256" },
          { name: "deadline", type: "uint256" },
          { name: "sellToken", type: "address" },
          { name: "buyToken", type: "address" },
          { name: "strategySellAmount", type: "uint256" },
          { name: "minStrategyBuyAmount", type: "uint256" },
          { name: "strategyRouterCalldataHash", type: "bytes32" },
          { name: "platformFeeUsdc", type: "uint256" },
          { name: "feeRecipient", type: "address" },
          { name: "gasFundingMode", type: "uint8" },
          { name: "gasTopUpUsdc", type: "uint256" },
          { name: "gasTopUpNative", type: "uint256" },
          { name: "gasRecipient", type: "address" },
          { name: "gasRouterCalldataHash", type: "bytes32" },
        ],
      },
      { name: "strategyRouterCalldata", type: "bytes" },
      { name: "gasRouterCalldata", type: "bytes" },
      { name: "sessionSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "event",
    name: "GasCreditFunded",
    inputs: [
      { name: "strategyId", type: "bytes32", indexed: true },
      { name: "sessionKey", type: "address", indexed: true },
      { name: "gasRecipient", type: "address", indexed: true },
      { name: "fundingMode", type: "uint8", indexed: false },
      { name: "gasTopUpUsdc", type: "uint256", indexed: false },
      { name: "nativeAmount", type: "uint256", indexed: false },
    ],
    anonymous: false,
  },
] as const

export type SwapIntentMessage = {
  strategyId: Hex
  sessionKey: Address
  nonce: bigint
  deadline: bigint
  sellToken: Address
  buyToken: Address
  maxSellAmount: bigint
  minBuyAmount: bigint
  routerCalldataHash: Hex
}

export type SwapBundleIntentMessage = {
  core: {
    strategyId: Hex
    sessionKey: Address
    nonce: bigint
    deadline: bigint
    sellToken: Address
    buyToken: Address
    maxSellAmount: bigint
    minBuyAmount: bigint
    routerCalldataHash: Hex
  }
  fees: {
    platformFeeUsdc: bigint
    feeRecipient: Address
    gasSellUsdc: bigint
    minNativeOut: bigint
    gasRecipient: Address
    gasRouterCalldataHash: Hex
  }
}

export type SwapBundleIntentV2Message = {
  strategyId: Hex
  sessionKey: Address
  nonce: bigint
  deadline: bigint
  sellToken: Address
  buyToken: Address
  strategySellAmount: bigint
  minStrategyBuyAmount: bigint
  strategyRouterCalldataHash: Hex
  platformFeeUsdc: bigint
  feeRecipient: Address
  gasFundingMode: GasFundingMode
  gasTopUpUsdc: bigint
  gasTopUpNative: bigint
  gasRecipient: Address
  gasRouterCalldataHash: Hex
}

export type RevokeIntentMessage = {
  strategyId: Hex
  sessionKey: Address
  nonce: bigint
  deadline: bigint
}

export function swapIntentTypes() {
  return {
    SwapIntent: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "sellToken", type: "address" },
      { name: "buyToken", type: "address" },
      { name: "maxSellAmount", type: "uint256" },
      { name: "minBuyAmount", type: "uint256" },
      { name: "routerCalldataHash", type: "bytes32" },
    ],
  } as const
}

export function swapBundleIntentTypes() {
  return {
    SwapBundleIntent: [
      { name: "core", type: "SwapBundleCore" },
      { name: "fees", type: "SwapBundleFees" },
    ],
    SwapBundleCore: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "sellToken", type: "address" },
      { name: "buyToken", type: "address" },
      { name: "maxSellAmount", type: "uint256" },
      { name: "minBuyAmount", type: "uint256" },
      { name: "routerCalldataHash", type: "bytes32" },
    ],
    SwapBundleFees: [
      { name: "platformFeeUsdc", type: "uint256" },
      { name: "feeRecipient", type: "address" },
      { name: "gasSellUsdc", type: "uint256" },
      { name: "minNativeOut", type: "uint256" },
      { name: "gasRecipient", type: "address" },
      { name: "gasRouterCalldataHash", type: "bytes32" },
    ],
  } as const
}

export function swapBundleIntentV2Types() {
  return {
    SwapBundleIntentV2: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "sellToken", type: "address" },
      { name: "buyToken", type: "address" },
      { name: "strategySellAmount", type: "uint256" },
      { name: "minStrategyBuyAmount", type: "uint256" },
      { name: "strategyRouterCalldataHash", type: "bytes32" },
      { name: "platformFeeUsdc", type: "uint256" },
      { name: "feeRecipient", type: "address" },
      { name: "gasFundingMode", type: "uint8" },
      { name: "gasTopUpUsdc", type: "uint256" },
      { name: "gasTopUpNative", type: "uint256" },
      { name: "gasRecipient", type: "address" },
      { name: "gasRouterCalldataHash", type: "bytes32" },
    ],
  } as const
}

export function swapBundleIntentDomain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return swapIntentDomain(input)
}

export function swapBundleIntentV2Domain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return {
    name: SESSION_SPEND_NAME,
    version: SESSION_SPEND_V2_VERSION,
    chainId: input.chainId,
    verifyingContract: input.verifyingContract,
  }
}

export function revokeIntentTypes() {
  return {
    RevokeIntent: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
    ],
  } as const
}

export function swapIntentDomain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return {
    name: SESSION_SPEND_NAME,
    version: SESSION_SPEND_VERSION,
    chainId: input.chainId,
    verifyingContract: input.verifyingContract,
  }
}

export function strategyIdFromCuid(strategyCuid: string): Hex {
  return keccak256(toBytes(strategyCuid))
}

export function encodeSessionGrant(input: {
  strategyId: Hex
  sessionAddress: Address
  limitUsdc: string
  expiresAt: number
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "grant",
    args: [
      input.strategyId,
      input.sessionAddress,
      parseUnits(input.limitUsdc, 6),
      BigInt(input.expiresAt),
    ],
  })
}

export function encodeSessionSetLimit(input: {
  strategyId: Hex
  sessionAddress: Address
  limitUsdc: string
  expiresAt: number
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "setLimit",
    args: [
      input.strategyId,
      input.sessionAddress,
      parseUnits(input.limitUsdc, 6),
      BigInt(input.expiresAt),
    ],
  })
}

export function encodeSessionRotate(input: {
  strategyId: Hex
  oldKey: Address
  newKey: Address
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "rotateSession",
    args: [input.strategyId, input.oldKey, input.newKey],
  })
}

export function encodeSessionRevoke(input: {
  strategyId: Hex
  sessionAddress: Address
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "revoke",
    args: [input.strategyId, input.sessionAddress],
  })
}

export function encodeSessionSignedRevoke(input: {
  intent: RevokeIntentMessage
  sessionSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "revokeWithSignature",
    args: [input.intent, input.sessionSignature],
  })
}

export function encodeExecuteSwap(input: {
  intent: SwapIntentMessage
  routerCalldata: Hex
  sessionSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "executeSwap",
    args: [
      {
        strategyId: input.intent.strategyId,
        sessionKey: input.intent.sessionKey,
        nonce: input.intent.nonce,
        deadline: input.intent.deadline,
        sellToken: input.intent.sellToken,
        buyToken: input.intent.buyToken,
        maxSellAmount: input.intent.maxSellAmount,
        minBuyAmount: input.intent.minBuyAmount,
        routerCalldataHash: input.intent.routerCalldataHash,
      },
      input.routerCalldata,
      input.sessionSignature,
    ],
  })
}

export function encodeExecuteSwapWithFees(input: {
  intent: SwapBundleIntentMessage
  strategyRouterCalldata: Hex
  gasRouterCalldata: Hex
  sessionSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "executeSwapWithFees",
    args: [
      {
        strategyId: input.intent.core.strategyId,
        sessionKey: input.intent.core.sessionKey,
        nonce: input.intent.core.nonce,
        deadline: input.intent.core.deadline,
        sellToken: input.intent.core.sellToken,
        buyToken: input.intent.core.buyToken,
        maxSellAmount: input.intent.core.maxSellAmount,
        minBuyAmount: input.intent.core.minBuyAmount,
        routerCalldataHash: input.intent.core.routerCalldataHash,
        platformFeeUsdc: input.intent.fees.platformFeeUsdc,
        feeRecipient: input.intent.fees.feeRecipient,
        gasSellUsdc: input.intent.fees.gasSellUsdc,
        minNativeOut: input.intent.fees.minNativeOut,
        gasRecipient: input.intent.fees.gasRecipient,
        gasRouterCalldataHash: input.intent.fees.gasRouterCalldataHash,
      },
      input.strategyRouterCalldata,
      input.gasRouterCalldata,
      input.sessionSignature,
    ],
  })
}

export function encodeExecuteSwapWithFeesV2(input: {
  intent: SwapBundleIntentV2Message
  strategyRouterCalldata: Hex
  gasRouterCalldata: Hex
  sessionSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "executeSwapWithFeesV2",
    args: [
      input.intent,
      input.strategyRouterCalldata,
      input.gasRouterCalldata,
      input.sessionSignature,
    ],
  })
}
