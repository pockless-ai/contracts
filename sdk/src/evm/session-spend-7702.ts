import {
  encodeAbiParameters,
  encodeFunctionData,
  getCreate2Address,
  keccak256,
  parseUnits,
  toBytes,
  type Address,
  type Hex,
} from "viem"

export const SESSION_SPEND_NAME = "PocklessSessionSpend7702"
export const SESSION_SPEND_VERSION = "1"
export const EVM_NATIVE_TOKEN = "0x0000000000000000000000000000000000000000"
export const ZEROX_NATIVE_TOKEN = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE"

const ORDER_ID_PATTERN = /^0x[0-9a-fA-F]{64}$/

export const GasFundingMode = {
  CREDIT_ONLY: 0,
  SEPARATE_TOPUP: 1,
  NATIVE_OUTPUT: 2,
} as const

export type GasFundingMode =
  (typeof GasFundingMode)[keyof typeof GasFundingMode]

export const RelayAction = {
  Deposit: 0,
  CreditAsset: 1,
  RemoteSell: 2,
  UsdcReturn: 3,
  DepositRelease: 4,
  AssetRestore: 5,
  GasTopUp: 6,
  GasTopUpRelease: 7,
} as const

export type RelayAction = (typeof RelayAction)[keyof typeof RelayAction]

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
    type: "function",
    name: "executeRelayDeposit",
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
          { name: "relayOrderId", type: "bytes32" },
          { name: "fundingChainId", type: "uint256" },
          { name: "originToken", type: "address" },
          { name: "originTokenDecimals", type: "uint8" },
          { name: "destToken", type: "address" },
          { name: "destTokenDecimals", type: "uint8" },
          { name: "originAmount", type: "uint256" },
          { name: "destChainId", type: "uint256" },
          { name: "minDestAmount", type: "uint256" },
          { name: "destRecipient", type: "address" },
          { name: "refundVault", type: "address" },
          { name: "relayCalldataHash", type: "bytes32" },
          { name: "platformFeeUsdc", type: "uint256" },
          { name: "feeRecipient", type: "address" },
        ],
      },
      { name: "relayTarget", type: "address" },
      { name: "relayCalldata", type: "bytes" },
      { name: "relayValue", type: "uint256" },
      { name: "sessionSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "creditRelayAsset",
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
          { name: "relayOrderId", type: "bytes32" },
          { name: "token", type: "address" },
          { name: "fundingChainId", type: "uint256" },
          { name: "creditQuantity", type: "uint256" },
          { name: "costUsdc", type: "uint128" },
        ],
      },
      { name: "sessionSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "executeRemoteRelaySell",
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
          { name: "relayOrderId", type: "bytes32" },
          { name: "token", type: "address" },
          { name: "fundingChainId", type: "uint256" },
          { name: "sellQuantity", type: "uint256" },
          { name: "minReturnUsdc", type: "uint256" },
          { name: "relayCalldataHash", type: "bytes32" },
        ],
      },
      { name: "relayTarget", type: "address" },
      { name: "relayCalldata", type: "bytes" },
      { name: "relayValue", type: "uint256" },
      { name: "sessionSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "creditUsdcReturn",
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
          { name: "relayOrderId", type: "bytes32" },
          { name: "fundingChainId", type: "uint256" },
          { name: "usdcReceived", type: "uint256" },
          { name: "destQuantityReleased", type: "uint256" },
          { name: "destCostReleasedUsdc", type: "uint128" },
          { name: "platformFeeUsdc", type: "uint256" },
          { name: "feeRecipient", type: "address" },
        ],
      },
      { name: "sessionSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "releaseRelayDeposit",
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
          { name: "relayOrderId", type: "bytes32" },
          { name: "token", type: "address" },
          { name: "fundingChainId", type: "uint256" },
          { name: "refundQuantity", type: "uint256" },
          { name: "refundCostUsdc", type: "uint128" },
        ],
      },
      { name: "sessionSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "restoreRemoteRelayAsset",
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
          { name: "relayOrderId", type: "bytes32" },
          { name: "token", type: "address" },
          { name: "fundingChainId", type: "uint256" },
          { name: "restoreQuantity", type: "uint256" },
          { name: "restoreCostUsdc", type: "uint128" },
        ],
      },
      { name: "sessionSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "executeRelayGasTopUp",
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
          { name: "relayOrderId", type: "bytes32" },
          { name: "overheadUsdc", type: "uint256" },
          { name: "gasRecipient", type: "address" },
          { name: "relayCalldataHash", type: "bytes32" },
        ],
      },
      { name: "relayTarget", type: "address" },
      { name: "relayCalldata", type: "bytes" },
      { name: "relayValue", type: "uint256" },
      { name: "sessionSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "releaseRelayGasTopUp",
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
          { name: "relayOrderId", type: "bytes32" },
          { name: "refundUsdc", type: "uint256" },
        ],
      },
      { name: "sessionSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "setPlatformRelayer",
    stateMutability: "nonpayable",
    inputs: [{ name: "relayer", type: "address" }],
    outputs: [],
  },
  {
    type: "function",
    name: "recoverVaultSurplus",
    stateMutability: "nonpayable",
    inputs: [
      { name: "strategyId", type: "bytes32" },
      { name: "token", type: "address" },
      { name: "recipient", type: "address" },
      { name: "amount", type: "uint256" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "walletRelayNonce",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "uint256" }],
  },
  {
    type: "function",
    name: "executeWalletRelaySwap",
    stateMutability: "payable",
    inputs: [
      {
        name: "intent",
        type: "tuple",
        components: [
          { name: "nonce", type: "uint256" },
          { name: "deadline", type: "uint256" },
          { name: "sellToken", type: "address" },
          { name: "sellAmount", type: "uint256" },
          { name: "destRecipient", type: "address" },
          { name: "relayCalldataHash", type: "bytes32" },
          { name: "relayOrderId", type: "bytes32" },
          { name: "platformFeeUsdc", type: "uint256" },
          { name: "feeRecipient", type: "address" },
        ],
      },
      { name: "relayTarget", type: "address" },
      { name: "relayCalldata", type: "bytes" },
      { name: "relayValue", type: "uint256" },
      { name: "ownerSignature", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "strategyVaultInitCodeHash",
    stateMutability: "pure",
    inputs: [{ name: "owner_", type: "address" }],
    outputs: [{ name: "", type: "bytes32" }],
  },
  {
    type: "function",
    name: "predictStrategyVault",
    stateMutability: "view",
    inputs: [{ name: "strategyId", type: "bytes32" }],
    outputs: [{ name: "", type: "address" }],
  },
  {
    type: "function",
    name: "platformRelayer",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "address" }],
  },
  {
    type: "function",
    name: "strategyVaultOf",
    stateMutability: "view",
    inputs: [{ name: "strategyId", type: "bytes32" }],
    outputs: [{ name: "", type: "address" }],
  },
] as const

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

export type RelayDepositIntentMessage = {
  strategyId: Hex
  sessionKey: Address
  nonce: bigint
  deadline: bigint
  relayOrderId: Hex
  fundingChainId: bigint
  originToken: Address
  originTokenDecimals: number
  destToken: Address
  destTokenDecimals: number
  originAmount: bigint
  destChainId: bigint
  minDestAmount: bigint
  destRecipient: Address
  refundVault: Address
  relayCalldataHash: Hex
  platformFeeUsdc: bigint
  feeRecipient: Address
}

export type CreditRelayAssetIntentMessage = {
  strategyId: Hex
  sessionKey: Address
  nonce: bigint
  deadline: bigint
  relayOrderId: Hex
  token: Address
  fundingChainId: bigint
  creditQuantity: bigint
  costUsdc: bigint
}

export type RemoteRelaySellIntentMessage = {
  strategyId: Hex
  sessionKey: Address
  nonce: bigint
  deadline: bigint
  relayOrderId: Hex
  token: Address
  fundingChainId: bigint
  sellQuantity: bigint
  minReturnUsdc: bigint
  relayCalldataHash: Hex
}

export type CreditUsdcReturnIntentMessage = {
  strategyId: Hex
  sessionKey: Address
  nonce: bigint
  deadline: bigint
  relayOrderId: Hex
  fundingChainId: bigint
  usdcReceived: bigint
  destQuantityReleased: bigint
  destCostReleasedUsdc: bigint
  platformFeeUsdc: bigint
  feeRecipient: Address
}

export type ReleaseRelayDepositIntentMessage = {
  strategyId: Hex
  sessionKey: Address
  nonce: bigint
  deadline: bigint
  relayOrderId: Hex
  token: Address
  fundingChainId: bigint
  refundQuantity: bigint
  refundCostUsdc: bigint
}

export type RestoreRemoteRelayAssetIntentMessage = {
  strategyId: Hex
  sessionKey: Address
  nonce: bigint
  deadline: bigint
  relayOrderId: Hex
  token: Address
  fundingChainId: bigint
  restoreQuantity: bigint
  restoreCostUsdc: bigint
}

export type RelayGasTopUpIntentMessage = {
  strategyId: Hex
  sessionKey: Address
  nonce: bigint
  deadline: bigint
  relayOrderId: Hex
  overheadUsdc: bigint
  gasRecipient: Address
  relayCalldataHash: Hex
}

export type ReleaseRelayGasTopUpIntentMessage = {
  strategyId: Hex
  sessionKey: Address
  nonce: bigint
  deadline: bigint
  relayOrderId: Hex
  refundUsdc: bigint
}

export type WalletRelaySwapIntentMessage = {
  nonce: bigint
  deadline: bigint
  sellToken: Address
  sellAmount: bigint
  destRecipient: Address
  relayCalldataHash: Hex
  relayOrderId: Hex
  platformFeeUsdc: bigint
  feeRecipient: Address
}

export type RevokeIntentMessage = {
  strategyId: Hex
  sessionKey: Address
  nonce: bigint
  deadline: bigint
}

export function sessionSpendDomain(input: {
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

export function swapBundleIntentV2Domain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return sessionSpendDomain(input)
}

export function relayDepositIntentTypes() {
  return {
    RelayDepositIntent: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "relayOrderId", type: "bytes32" },
      { name: "fundingChainId", type: "uint256" },
      { name: "originToken", type: "address" },
      { name: "originTokenDecimals", type: "uint8" },
      { name: "destToken", type: "address" },
      { name: "destTokenDecimals", type: "uint8" },
      { name: "originAmount", type: "uint256" },
      { name: "destChainId", type: "uint256" },
      { name: "minDestAmount", type: "uint256" },
      { name: "destRecipient", type: "address" },
      { name: "refundVault", type: "address" },
      { name: "relayCalldataHash", type: "bytes32" },
      { name: "platformFeeUsdc", type: "uint256" },
      { name: "feeRecipient", type: "address" },
    ],
  } as const
}

export function creditRelayAssetIntentTypes() {
  return {
    CreditRelayAssetIntent: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "relayOrderId", type: "bytes32" },
      { name: "token", type: "address" },
      { name: "fundingChainId", type: "uint256" },
      { name: "creditQuantity", type: "uint256" },
      { name: "costUsdc", type: "uint128" },
    ],
  } as const
}

export function remoteRelaySellIntentTypes() {
  return {
    RemoteRelaySellIntent: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "relayOrderId", type: "bytes32" },
      { name: "token", type: "address" },
      { name: "fundingChainId", type: "uint256" },
      { name: "sellQuantity", type: "uint256" },
      { name: "minReturnUsdc", type: "uint256" },
      { name: "relayCalldataHash", type: "bytes32" },
    ],
  } as const
}

export function creditUsdcReturnIntentTypes() {
  return {
    CreditUsdcReturnIntent: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "relayOrderId", type: "bytes32" },
      { name: "fundingChainId", type: "uint256" },
      { name: "usdcReceived", type: "uint256" },
      { name: "destQuantityReleased", type: "uint256" },
      { name: "destCostReleasedUsdc", type: "uint128" },
      { name: "platformFeeUsdc", type: "uint256" },
      { name: "feeRecipient", type: "address" },
    ],
  } as const
}

export function releaseRelayDepositIntentTypes() {
  return {
    ReleaseRelayDepositIntent: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "relayOrderId", type: "bytes32" },
      { name: "token", type: "address" },
      { name: "fundingChainId", type: "uint256" },
      { name: "refundQuantity", type: "uint256" },
      { name: "refundCostUsdc", type: "uint128" },
    ],
  } as const
}

export function restoreRemoteRelayAssetIntentTypes() {
  return {
    RestoreRemoteRelayAssetIntent: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "relayOrderId", type: "bytes32" },
      { name: "token", type: "address" },
      { name: "fundingChainId", type: "uint256" },
      { name: "restoreQuantity", type: "uint256" },
      { name: "restoreCostUsdc", type: "uint128" },
    ],
  } as const
}

export function relayGasTopUpIntentTypes() {
  return {
    RelayGasTopUpIntent: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "relayOrderId", type: "bytes32" },
      { name: "overheadUsdc", type: "uint256" },
      { name: "gasRecipient", type: "address" },
      { name: "relayCalldataHash", type: "bytes32" },
    ],
  } as const
}

export function releaseRelayGasTopUpIntentTypes() {
  return {
    ReleaseRelayGasTopUpIntent: [
      { name: "strategyId", type: "bytes32" },
      { name: "sessionKey", type: "address" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "relayOrderId", type: "bytes32" },
      { name: "refundUsdc", type: "uint256" },
    ],
  } as const
}

export function relayDepositIntentDomain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return sessionSpendDomain(input)
}

export function creditRelayAssetIntentDomain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return sessionSpendDomain(input)
}

export function remoteRelaySellIntentDomain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return sessionSpendDomain(input)
}

export function creditUsdcReturnIntentDomain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return sessionSpendDomain(input)
}

export function releaseRelayDepositIntentDomain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return sessionSpendDomain(input)
}

export function restoreRemoteRelayAssetIntentDomain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return sessionSpendDomain(input)
}

export function relayGasTopUpIntentDomain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return sessionSpendDomain(input)
}

export function releaseRelayGasTopUpIntentDomain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return sessionSpendDomain(input)
}

export function walletRelaySwapIntentTypes() {
  return {
    WalletRelaySwapIntent: [
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "sellToken", type: "address" },
      { name: "sellAmount", type: "uint256" },
      { name: "destRecipient", type: "address" },
      { name: "relayCalldataHash", type: "bytes32" },
      { name: "relayOrderId", type: "bytes32" },
      { name: "platformFeeUsdc", type: "uint256" },
      { name: "feeRecipient", type: "address" },
    ],
  } as const
}

export function walletRelaySwapIntentDomain(input: {
  chainId: number
  verifyingContract: Address
}) {
  return sessionSpendDomain(input)
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

export function hashRelayCalldata(input: {
  relayTarget: Address
  relayValue: bigint
  relayCalldata: Hex
}) {
  return keccak256(
    encodeAbiParameters(
      [
        { type: "address" },
        { type: "uint256" },
        { type: "bytes" },
      ],
      [input.relayTarget, input.relayValue, input.relayCalldata]
    )
  )
}

export function normalizeRelayOrderId(value: string): Hex {
  const trimmed = value.trim()
  if (!ORDER_ID_PATTERN.test(trimmed)) {
    throw new Error(`Invalid Relay orderId: ${value}`)
  }
  return trimmed as Hex
}

export function strategyVaultSalt(strategyId: Hex): Hex {
  return keccak256(encodeAbiParameters([{ type: "bytes32" }], [strategyId]))
}

export function strategyVaultAddress(input: {
  sessionSpend: Address
  strategyId: Hex
  initCodeHash: Hex
}): Address {
  return getCreate2Address({
    bytecodeHash: input.initCodeHash,
    from: input.sessionSpend,
    salt: strategyVaultSalt(input.strategyId),
  })
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

export function encodeExecuteRelayDeposit(input: {
  intent: RelayDepositIntentMessage
  relayTarget: Address
  relayCalldata: Hex
  relayValue: bigint
  sessionSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "executeRelayDeposit",
    args: [
      input.intent,
      input.relayTarget,
      input.relayCalldata,
      input.relayValue,
      input.sessionSignature,
    ],
  })
}

export function encodeCreditRelayAsset(input: {
  intent: CreditRelayAssetIntentMessage
  sessionSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "creditRelayAsset",
    args: [input.intent, input.sessionSignature],
  })
}

export function encodeExecuteRemoteRelaySell(input: {
  intent: RemoteRelaySellIntentMessage
  relayTarget: Address
  relayCalldata: Hex
  relayValue: bigint
  sessionSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "executeRemoteRelaySell",
    args: [
      input.intent,
      input.relayTarget,
      input.relayCalldata,
      input.relayValue,
      input.sessionSignature,
    ],
  })
}

export function encodeCreditUsdcReturn(input: {
  intent: CreditUsdcReturnIntentMessage
  sessionSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "creditUsdcReturn",
    args: [input.intent, input.sessionSignature],
  })
}

export function encodeReleaseRelayDeposit(input: {
  intent: ReleaseRelayDepositIntentMessage
  sessionSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "releaseRelayDeposit",
    args: [input.intent, input.sessionSignature],
  })
}

export function encodeRestoreRemoteRelayAsset(input: {
  intent: RestoreRemoteRelayAssetIntentMessage
  sessionSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "restoreRemoteRelayAsset",
    args: [input.intent, input.sessionSignature],
  })
}

export function encodeExecuteRelayGasTopUp(input: {
  intent: RelayGasTopUpIntentMessage
  relayTarget: Address
  relayCalldata: Hex
  relayValue: bigint
  sessionSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "executeRelayGasTopUp",
    args: [
      input.intent,
      input.relayTarget,
      input.relayCalldata,
      input.relayValue,
      input.sessionSignature,
    ],
  })
}

export function encodeReleaseRelayGasTopUp(input: {
  intent: ReleaseRelayGasTopUpIntentMessage
  sessionSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "releaseRelayGasTopUp",
    args: [input.intent, input.sessionSignature],
  })
}

export function encodeSetPlatformRelayer(relayer: Address) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "setPlatformRelayer",
    args: [relayer],
  })
}

export function encodeRecoverVaultSurplus(input: {
  strategyId: Hex
  token: Address
  recipient: Address
  amount: bigint
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "recoverVaultSurplus",
    args: [input.strategyId, input.token, input.recipient, input.amount],
  })
}

export function encodeExecuteWalletRelaySwap(input: {
  intent: WalletRelaySwapIntentMessage
  relayTarget: Address
  relayCalldata: Hex
  relayValue: bigint
  ownerSignature: Hex
}) {
  return encodeFunctionData({
    abi: sessionSpend7702Abi,
    functionName: "executeWalletRelaySwap",
    args: [
      input.intent,
      input.relayTarget,
      input.relayCalldata,
      input.relayValue,
      input.ownerSignature,
    ],
  })
}
