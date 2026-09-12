// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {StrategyVault} from "src/StrategyVault.sol";

interface IERC20Extended {
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
    function transfer(address to, uint256 amount) external returns (bool);
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
    function decimals() external view returns (uint8);
}

struct SwapBundleIntentV2Payload {
    bytes32 strategyId;
    address sessionKey;
    uint256 nonce;
    uint256 deadline;
    address sellToken;
    address buyToken;
    uint256 strategySellAmount;
    uint256 minStrategyBuyAmount;
    bytes32 strategyRouterCalldataHash;
    uint256 platformFeeUsdc;
    address feeRecipient;
    uint8 gasFundingMode;
    uint256 gasTopUpUsdc;
    uint256 gasTopUpNative;
    address gasRecipient;
    bytes32 gasRouterCalldataHash;
}

struct RelayDepositIntentPayload {
    bytes32 strategyId;
    address sessionKey;
    uint256 nonce;
    uint256 deadline;
    bytes32 relayOrderId;
    uint256 fundingChainId;
    address originToken;
    uint8 originTokenDecimals;
    address destToken;
    uint8 destTokenDecimals;
    uint256 originAmount;
    uint256 destChainId;
    uint256 minDestAmount;
    address destRecipient;
    address refundVault;
    bytes32 relayCalldataHash;
    uint256 platformFeeUsdc;
    address feeRecipient;
}

struct CreditRelayAssetIntentPayload {
    bytes32 strategyId;
    address sessionKey;
    uint256 nonce;
    uint256 deadline;
    bytes32 relayOrderId;
    address token;
    uint256 fundingChainId;
    uint256 creditQuantity;
    uint128 costUsdc;
}

struct RemoteRelaySellIntentPayload {
    bytes32 strategyId;
    address sessionKey;
    uint256 nonce;
    uint256 deadline;
    bytes32 relayOrderId;
    address token;
    uint256 fundingChainId;
    uint256 sellQuantity;
    uint256 minReturnUsdc;
    bytes32 relayCalldataHash;
}

struct CreditUsdcReturnIntentPayload {
    bytes32 strategyId;
    address sessionKey;
    uint256 nonce;
    uint256 deadline;
    bytes32 relayOrderId;
    uint256 fundingChainId;
    uint256 usdcReceived;
    uint256 destQuantityReleased;
    uint128 destCostReleasedUsdc;
    uint256 platformFeeUsdc;
    address feeRecipient;
}

struct ReleaseRelayDepositIntentPayload {
    bytes32 strategyId;
    address sessionKey;
    uint256 nonce;
    uint256 deadline;
    bytes32 relayOrderId;
    address token;
    uint256 fundingChainId;
    uint256 refundQuantity;
    uint128 refundCostUsdc;
}

struct RestoreRemoteRelayAssetIntentPayload {
    bytes32 strategyId;
    address sessionKey;
    uint256 nonce;
    uint256 deadline;
    bytes32 relayOrderId;
    address token;
    uint256 fundingChainId;
    uint256 restoreQuantity;
    uint128 restoreCostUsdc;
}

struct RelayGasTopUpIntentPayload {
    bytes32 strategyId;
    address sessionKey;
    uint256 nonce;
    uint256 deadline;
    bytes32 relayOrderId;
    uint256 overheadUsdc;
    address gasRecipient;
    bytes32 relayCalldataHash;
}

struct ReleaseRelayGasTopUpIntentPayload {
    bytes32 strategyId;
    address sessionKey;
    uint256 nonce;
    uint256 deadline;
    bytes32 relayOrderId;
    uint256 refundUsdc;
}

struct WalletRelaySwapIntentPayload {
    uint256 nonce;
    uint256 deadline;
    address sellToken;
    uint256 sellAmount;
    address destRecipient;
    bytes32 relayCalldataHash;
    bytes32 relayOrderId;
    uint256 platformFeeUsdc;
    address feeRecipient;
}

library SwapBundleIntentV2Hash {
    bytes32 internal constant TYPEHASH = keccak256(
        "SwapBundleIntentV2(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,address sellToken,address buyToken,uint256 strategySellAmount,uint256 minStrategyBuyAmount,bytes32 strategyRouterCalldataHash,uint256 platformFeeUsdc,address feeRecipient,uint8 gasFundingMode,uint256 gasTopUpUsdc,uint256 gasTopUpNative,address gasRecipient,bytes32 gasRouterCalldataHash)"
    );

    function digest(SwapBundleIntentV2Payload memory intent, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        bytes memory firstHalf = abi.encode(
            TYPEHASH,
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            intent.sellToken,
            intent.buyToken,
            intent.strategySellAmount,
            intent.minStrategyBuyAmount
        );
        bytes memory secondHalf = abi.encode(
            intent.strategyRouterCalldataHash,
            intent.platformFeeUsdc,
            intent.feeRecipient,
            intent.gasFundingMode,
            intent.gasTopUpUsdc,
            intent.gasTopUpNative,
            intent.gasRecipient,
            intent.gasRouterCalldataHash
        );
        return keccak256(
            abi.encodePacked(
                "\x19\x01", domainSeparator, keccak256(bytes.concat(firstHalf, secondHalf))
            )
        );
    }
}

library RelayDepositIntentHash {
    bytes32 internal constant TYPEHASH = keccak256(
        "RelayDepositIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,uint256 fundingChainId,address originToken,uint8 originTokenDecimals,address destToken,uint8 destTokenDecimals,uint256 originAmount,uint256 destChainId,uint256 minDestAmount,address destRecipient,address refundVault,bytes32 relayCalldataHash,uint256 platformFeeUsdc,address feeRecipient)"
    );

    function digest(RelayDepositIntentPayload memory intent, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        bytes memory firstHalf = abi.encode(
            TYPEHASH,
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            intent.relayOrderId,
            intent.fundingChainId,
            intent.originToken,
            intent.originTokenDecimals,
            intent.destToken,
            intent.destTokenDecimals
        );
        bytes memory secondHalf = abi.encode(
            intent.originAmount,
            intent.destChainId,
            intent.minDestAmount,
            intent.destRecipient,
            intent.refundVault,
            intent.relayCalldataHash,
            intent.platformFeeUsdc,
            intent.feeRecipient
        );
        return keccak256(
            abi.encodePacked(
                "\x19\x01", domainSeparator, keccak256(bytes.concat(firstHalf, secondHalf))
            )
        );
    }
}

library CreditRelayAssetIntentHash {
    bytes32 internal constant TYPEHASH = keccak256(
        "CreditRelayAssetIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,address token,uint256 fundingChainId,uint256 creditQuantity,uint128 costUsdc)"
    );

    function digest(CreditRelayAssetIntentPayload memory intent, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(
            abi.encodePacked(
                "\x19\x01",
                domainSeparator,
                keccak256(
                    abi.encode(
                        TYPEHASH,
                        intent.strategyId,
                        intent.sessionKey,
                        intent.nonce,
                        intent.deadline,
                        intent.relayOrderId,
                        intent.token,
                        intent.fundingChainId,
                        intent.creditQuantity,
                        intent.costUsdc
                    )
                )
            )
        );
    }
}

library RemoteRelaySellIntentHash {
    bytes32 internal constant TYPEHASH = keccak256(
        "RemoteRelaySellIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,address token,uint256 fundingChainId,uint256 sellQuantity,uint256 minReturnUsdc,bytes32 relayCalldataHash)"
    );

    function digest(RemoteRelaySellIntentPayload memory intent, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(
            abi.encodePacked(
                "\x19\x01",
                domainSeparator,
                keccak256(
                    abi.encode(
                        TYPEHASH,
                        intent.strategyId,
                        intent.sessionKey,
                        intent.nonce,
                        intent.deadline,
                        intent.relayOrderId,
                        intent.token,
                        intent.fundingChainId,
                        intent.sellQuantity,
                        intent.minReturnUsdc,
                        intent.relayCalldataHash
                    )
                )
            )
        );
    }
}

library CreditUsdcReturnIntentHash {
    bytes32 internal constant TYPEHASH = keccak256(
        "CreditUsdcReturnIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,uint256 fundingChainId,uint256 usdcReceived,uint256 destQuantityReleased,uint128 destCostReleasedUsdc,uint256 platformFeeUsdc,address feeRecipient)"
    );

    function digest(CreditUsdcReturnIntentPayload memory intent, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        bytes memory firstHalf = abi.encode(
            TYPEHASH,
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            intent.relayOrderId,
            intent.fundingChainId,
            intent.usdcReceived,
            intent.destQuantityReleased
        );
        bytes memory secondHalf =
            abi.encode(intent.destCostReleasedUsdc, intent.platformFeeUsdc, intent.feeRecipient);
        return keccak256(
            abi.encodePacked(
                "\x19\x01", domainSeparator, keccak256(bytes.concat(firstHalf, secondHalf))
            )
        );
    }
}

library ReleaseRelayDepositIntentHash {
    bytes32 internal constant TYPEHASH = keccak256(
        "ReleaseRelayDepositIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,address token,uint256 fundingChainId,uint256 refundQuantity,uint128 refundCostUsdc)"
    );

    function digest(ReleaseRelayDepositIntentPayload memory intent, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(
            abi.encodePacked(
                "\x19\x01",
                domainSeparator,
                keccak256(
                    abi.encode(
                        TYPEHASH,
                        intent.strategyId,
                        intent.sessionKey,
                        intent.nonce,
                        intent.deadline,
                        intent.relayOrderId,
                        intent.token,
                        intent.fundingChainId,
                        intent.refundQuantity,
                        intent.refundCostUsdc
                    )
                )
            )
        );
    }
}

library RestoreRemoteRelayAssetIntentHash {
    bytes32 internal constant TYPEHASH = keccak256(
        "RestoreRemoteRelayAssetIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,address token,uint256 fundingChainId,uint256 restoreQuantity,uint128 restoreCostUsdc)"
    );

    function digest(RestoreRemoteRelayAssetIntentPayload memory intent, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(
            abi.encodePacked(
                "\x19\x01",
                domainSeparator,
                keccak256(
                    abi.encode(
                        TYPEHASH,
                        intent.strategyId,
                        intent.sessionKey,
                        intent.nonce,
                        intent.deadline,
                        intent.relayOrderId,
                        intent.token,
                        intent.fundingChainId,
                        intent.restoreQuantity,
                        intent.restoreCostUsdc
                    )
                )
            )
        );
    }
}

library RelayGasTopUpIntentHash {
    bytes32 internal constant TYPEHASH = keccak256(
        "RelayGasTopUpIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,uint256 overheadUsdc,address gasRecipient,bytes32 relayCalldataHash)"
    );

    function digest(RelayGasTopUpIntentPayload memory intent, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(
            abi.encodePacked(
                "\x19\x01",
                domainSeparator,
                keccak256(
                    abi.encode(
                        TYPEHASH,
                        intent.strategyId,
                        intent.sessionKey,
                        intent.nonce,
                        intent.deadline,
                        intent.relayOrderId,
                        intent.overheadUsdc,
                        intent.gasRecipient,
                        intent.relayCalldataHash
                    )
                )
            )
        );
    }
}

library ReleaseRelayGasTopUpIntentHash {
    bytes32 internal constant TYPEHASH = keccak256(
        "ReleaseRelayGasTopUpIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,uint256 refundUsdc)"
    );

    function digest(ReleaseRelayGasTopUpIntentPayload memory intent, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(
            abi.encodePacked(
                "\x19\x01",
                domainSeparator,
                keccak256(
                    abi.encode(
                        TYPEHASH,
                        intent.strategyId,
                        intent.sessionKey,
                        intent.nonce,
                        intent.deadline,
                        intent.relayOrderId,
                        intent.refundUsdc
                    )
                )
            )
        );
    }
}

library WalletRelaySwapIntentHash {
    bytes32 internal constant TYPEHASH = keccak256(
        "WalletRelaySwapIntent(uint256 nonce,uint256 deadline,address sellToken,uint256 sellAmount,address destRecipient,bytes32 relayCalldataHash,bytes32 relayOrderId,uint256 platformFeeUsdc,address feeRecipient)"
    );

    function digest(WalletRelaySwapIntentPayload memory intent, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(
            abi.encodePacked(
                "\x19\x01",
                domainSeparator,
                keccak256(
                    abi.encode(
                        TYPEHASH,
                        intent.nonce,
                        intent.deadline,
                        intent.sellToken,
                        intent.sellAmount,
                        intent.destRecipient,
                        intent.relayCalldataHash,
                        intent.relayOrderId,
                        intent.platformFeeUsdc,
                        intent.feeRecipient
                    )
                )
            )
        );
    }
}

/// @title SessionSpendBase
/// @notice ERC-7702 implementation for per-strategy session keys that may swap
///         through a pinned 0x AllowanceHolder. Storage is ERC-7201 namespaced.
abstract contract SessionSpendBase {
    /// keccak256(abi.encode(uint256(keccak256("pockless.session.spend7702.v1")) - 1)) & ~bytes32(uint256(0xff))
    bytes32 private constant STORAGE_LOCATION =
        0x3e0c859c46df804f27f96dac030007e24ba5d79c9f807df46118f59ed197e100;

    address public constant ALLOWANCE_HOLDER = 0x0000000000001fF3684f28c67538d4D072C22734;
    address public constant ZEROX_NATIVE_TOKEN = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;

    address private constant RELAY_ERC20_ROUTER = 0xb92fe925DC43a0ECdE6c8b1a2709c170Ec4fFf4f;
    address private constant RELAY_APPROVAL_PROXY = 0xCcC88a9d1B4ED6b0EABA998850414b24f1c315bE;
    address private constant RELAY_DEPOSITORY = 0x4cD00E387622C35bDDB9b4c962C136462338BC31;
    address private constant RELAY_DEPOSITORY_ALT = 0x59916DA825D2D2eC1BF878D71c88826F6633ecca;
    address private constant RELAY_ERC20_ROUTER_ALT = 0x9EF6d3c2F60d7b9008D74CAb1FC0F899c957C819;
    address private constant RELAY_APPROVAL_PROXY_ALT = 0x8754Bc615047de01228a7527B712806A71A8dc9a;
    address private constant RELAY_RECEIVER = 0xa5F565650890fBA1824Ee0F21EbBbF660a179934;
    address private constant RELAY_MULTICALLER = 0x0000000000002Bdbf1Bf3279983603Ec279CC6dF;

    bytes4 private constant EXEC_SELECTOR = 0x2213bc0b;

    bytes32 private constant REVOKE_INTENT_TYPEHASH = keccak256(
        "RevokeIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline)"
    );

    address public immutable usdcToken;
    uint8 public immutable usdcDecimals;

    struct Session {
        uint128 limitUsdc;
        uint128 capacityUsdc;
        uint128 deployedUsdc;
        uint64 expiresAt;
        uint64 nonce;
        bool revoked;
        bool exists;
    }

    struct AssetRecord {
        uint256 quantity;
        uint128 costUsdc;
    }

    struct RemoteAssetRecord {
        uint256 quantity;
        uint128 costUsdc;
    }

    struct PendingDeposit {
        bytes32 strategyId;
        uint128 lockedCostUsdc;
        address refundVault;
        uint256 originAmount;
        address originToken;
        uint256 fundingChainId;
        bool exists;
    }

    struct PendingSell {
        bytes32 strategyId;
        address token;
        uint256 fundingChainId;
        uint256 quantity;
        uint128 provisionalCostUsdc;
        bool exists;
    }

    struct PendingGasTopUp {
        bytes32 strategyId;
        uint128 overheadUsdc;
        address gasRecipient;
        bool exists;
    }

    enum GasFundingMode {
        CREDIT_ONLY,
        SEPARATE_TOPUP,
        NATIVE_OUTPUT
    }

    enum RelayAction {
        Deposit,
        CreditAsset,
        RemoteSell,
        UsdcReturn,
        DepositRelease,
        AssetRestore,
        GasTopUp,
        GasTopUpRelease
    }

    struct SwapBundleIntentV2 {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
        address sellToken;
        address buyToken;
        uint256 strategySellAmount;
        uint256 minStrategyBuyAmount;
        bytes32 strategyRouterCalldataHash;
        uint256 platformFeeUsdc;
        address feeRecipient;
        GasFundingMode gasFundingMode;
        uint256 gasTopUpUsdc;
        uint256 gasTopUpNative;
        address gasRecipient;
        bytes32 gasRouterCalldataHash;
    }

    struct RevokeIntent {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
    }

    struct RelayDepositIntent {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
        bytes32 relayOrderId;
        uint256 fundingChainId;
        address originToken;
        uint8 originTokenDecimals;
        address destToken;
        uint8 destTokenDecimals;
        uint256 originAmount;
        uint256 destChainId;
        uint256 minDestAmount;
        address destRecipient;
        address refundVault;
        bytes32 relayCalldataHash;
        uint256 platformFeeUsdc;
        address feeRecipient;
    }

    struct CreditRelayAssetIntent {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
        bytes32 relayOrderId;
        address token;
        uint256 fundingChainId;
        uint256 creditQuantity;
        uint128 costUsdc;
    }

    struct RemoteRelaySellIntent {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
        bytes32 relayOrderId;
        address token;
        uint256 fundingChainId;
        uint256 sellQuantity;
        uint256 minReturnUsdc;
        bytes32 relayCalldataHash;
    }

    struct CreditUsdcReturnIntent {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
        bytes32 relayOrderId;
        uint256 fundingChainId;
        uint256 usdcReceived;
        uint256 destQuantityReleased;
        uint128 destCostReleasedUsdc;
        uint256 platformFeeUsdc;
        address feeRecipient;
    }

    struct ReleaseRelayDepositIntent {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
        bytes32 relayOrderId;
        address token;
        uint256 fundingChainId;
        uint256 refundQuantity;
        uint128 refundCostUsdc;
    }

    struct RestoreRemoteRelayAssetIntent {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
        bytes32 relayOrderId;
        address token;
        uint256 fundingChainId;
        uint256 restoreQuantity;
        uint128 restoreCostUsdc;
    }

    struct RelayGasTopUpIntent {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
        bytes32 relayOrderId;
        uint256 overheadUsdc;
        address gasRecipient;
        bytes32 relayCalldataHash;
    }

    struct ReleaseRelayGasTopUpIntent {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
        bytes32 relayOrderId;
        uint256 refundUsdc;
    }

    struct WalletRelaySwapIntent {
        uint256 nonce;
        uint256 deadline;
        address sellToken;
        uint256 sellAmount;
        address destRecipient;
        bytes32 relayCalldataHash;
        bytes32 relayOrderId;
        uint256 platformFeeUsdc;
        address feeRecipient;
    }

    struct Layout {
        uint256 locked;
        bytes32[] strategyIds;
        mapping(bytes32 => uint256) strategyIndex;
        mapping(bytes32 => address[]) sessionKeys;
        mapping(bytes32 => mapping(address => uint256)) sessionKeyIndex;
        mapping(bytes32 => mapping(address => Session)) sessions;
        mapping(bytes32 => mapping(address => AssetRecord)) assets;
        uint256 walletRelayNonce;
        address platformRelayer;
        mapping(bytes32 => address) strategyVault;
        mapping(bytes32 => mapping(address => mapping(uint256 => RemoteAssetRecord))) remoteAssets;
        mapping(bytes32 => mapping(address => uint256)) vaultAccountedBalance;
        mapping(bytes32 => mapping(bytes32 => mapping(uint8 => bool))) consumedRelayReceipts;
        mapping(bytes32 => PendingDeposit) pendingDeposits;
        mapping(bytes32 => PendingSell) pendingSells;
        mapping(bytes32 => PendingGasTopUp) pendingGasTopUps;
        mapping(bytes32 => bool) walletRelayOrderConsumed;
    }

    error NotOwner();
    error NotPlatformRelayer();
    error SessionUnknown();
    error SessionExpired();
    error SessionRevoked();
    error InvalidSignature();
    error InvalidIntent();
    error IntentExpired();
    error NonceMismatch();
    error SelectorNotAllowed();
    error SpendLimitExceeded();
    error InsufficientInventory();
    error InsufficientVaultSurplus();
    error SlippageExceeded();
    error ZeroKey();
    error ZeroStrategy();
    error Reentrant();
    error CallFailed(bytes data);
    error InvalidLimit();
    error SessionKeyTaken();
    error SessionAlreadyExists();
    error RouterFieldsMismatch();
    error ApprovalFailed(address token, uint256 amount);
    error RelayReceiptConsumed();
    error PendingRecordMissing();

    event SessionGranted(
        bytes32 indexed strategyId, address indexed sessionKey, uint256 limitUsdc, uint256 expiresAt
    );
    event SessionRevocation(bytes32 indexed strategyId, address indexed sessionKey, uint64 nonce);
    event SessionRotated(
        bytes32 indexed strategyId, address indexed oldKey, address indexed newKey
    );
    event SessionLimitUpdated(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        uint256 oldLimitUsdc,
        uint256 newLimitUsdc,
        uint256 expiresAt
    );
    event SwapExecuted(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        address sellToken,
        address buyToken,
        uint256 sellAmount,
        uint256 buyAmount,
        int256 realizedPnlUsdc
    );
    event PlatformFeeCharged(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        address indexed feeRecipient,
        uint256 platformFeeUsdc
    );
    event GasCreditFunded(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        address indexed gasRecipient,
        GasFundingMode fundingMode,
        uint256 gasTopUpUsdc,
        uint256 nativeAmount
    );
    event StrategyVaultDeployed(bytes32 indexed strategyId, address indexed vault);
    event PlatformRelayerUpdated(address indexed oldRelayer, address indexed newRelayer);
    event RelayDepositExecuted(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        bytes32 indexed relayOrderId,
        address originToken,
        uint256 originAmount,
        uint256 destChainId,
        address destToken,
        uint256 minDestAmount,
        address destRecipient,
        address refundVault
    );
    event RelayAssetCredited(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        bytes32 indexed relayOrderId,
        address token,
        uint256 fundingChainId,
        uint256 creditQuantity,
        uint128 costUsdc
    );
    event RemoteRelaySellExecuted(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        bytes32 indexed relayOrderId,
        address token,
        uint256 fundingChainId,
        uint256 sellQuantity,
        uint128 provisionalCostUsdc
    );
    event UsdcReturnCredited(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        bytes32 indexed relayOrderId,
        uint256 usdcReceived,
        uint256 destQuantityReleased,
        uint128 destCostReleasedUsdc,
        int256 realizedPnlUsdc
    );
    event RelayDepositReleased(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        bytes32 indexed relayOrderId,
        uint256 refundQuantity,
        uint128 refundCostUsdc
    );
    event RemoteRelayAssetRestored(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        bytes32 indexed relayOrderId,
        address token,
        uint256 restoreQuantity,
        uint128 restoreCostUsdc
    );
    event RelayGasTopUpExecuted(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        bytes32 indexed relayOrderId,
        uint256 overheadUsdc,
        address gasRecipient
    );
    event RelayGasTopUpReleased(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        bytes32 indexed relayOrderId,
        uint256 refundUsdc
    );
    event VaultSurplusRecovered(
        bytes32 indexed strategyId, address indexed token, address indexed recipient, uint256 amount
    );
    event WalletRelaySwapExecuted(
        uint256 indexed nonce,
        address sellToken,
        uint256 sellAmount,
        address destRecipient,
        bytes32 relayOrderId
    );
    event WalletPlatformFeeCharged(address indexed feeRecipient, uint256 platformFeeUsdc);

    modifier onlyOwner() {
        if (msg.sender != address(this)) revert NotOwner();
        _;
    }

    modifier onlyPlatformRelayer() {
        if (msg.sender != _layout().platformRelayer) revert NotPlatformRelayer();
        _;
    }

    modifier nonReentrant() {
        Layout storage $ = _layout();
        if ($.locked == 1) revert Reentrant();
        $.locked = 1;
        _;
        $.locked = 0;
    }

    constructor(address usdcToken_) {
        if (usdcToken_ == address(0)) revert InvalidIntent();
        usdcToken = usdcToken_;
        usdcDecimals = IERC20Extended(usdcToken_).decimals();
    }

    receive() external payable {}

    function _emitRelayDepositExecuted(RelayDepositIntent calldata intent) internal {
        emit RelayDepositExecuted(
            intent.strategyId,
            intent.sessionKey,
            intent.relayOrderId,
            intent.originToken,
            intent.originAmount,
            intent.destChainId,
            intent.destToken,
            intent.minDestAmount,
            intent.destRecipient,
            intent.refundVault
        );
    }

    function _validateRemoteRelaySell(
        RemoteRelaySellIntent calldata intent,
        address relayTarget,
        bytes calldata relayCalldata,
        uint256 relayValue,
        bytes calldata sessionSignature
    ) internal returns (address vault) {
        bytes32 relayCallHash = keccak256(abi.encode(relayTarget, relayValue, relayCalldata));
        if (relayCallHash != intent.relayCalldataHash) revert InvalidIntent();
        _validateRelayTarget(relayTarget);
        if (intent.relayOrderId == bytes32(0) || intent.sellQuantity == 0) revert InvalidIntent();

        _validateSignedSwap(
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            RemoteRelaySellIntentHash.digest(_remoteRelaySellPayload(intent), _domainSeparator()),
            sessionSignature
        );
        _consumeRelayReceipt(intent.strategyId, intent.relayOrderId, RelayAction.RemoteSell);
        vault = _requireStrategyVault(_layout(), intent.strategyId);
    }

    function _stageRemoteRelaySell(RemoteRelaySellIntent calldata intent)
        internal
        returns (uint128 provisionalCost)
    {
        bytes32 strategyId = intent.strategyId;
        address token = intent.token;
        uint256 fundingChainId = intent.fundingChainId;
        uint256 sellQuantity = intent.sellQuantity;
        bytes32 relayOrderId = intent.relayOrderId;

        Layout storage $ = _layout();
        provisionalCost =
            _consumeRemoteInventory($, strategyId, token, fundingChainId, sellQuantity);
        _decreaseVaultAccounted($, strategyId, token, sellQuantity);
        $.pendingSells[relayOrderId] = PendingSell({
            strategyId: strategyId,
            token: token,
            fundingChainId: fundingChainId,
            quantity: sellQuantity,
            provisionalCostUsdc: provisionalCost,
            exists: true
        });
    }

    function _finalizeRemoteRelaySell(
        RemoteRelaySellIntent calldata intent,
        uint128 provisionalCost
    ) internal {
        Session storage session = _layout().sessions[intent.strategyId][intent.sessionKey];
        session.nonce += 1;
        emit RemoteRelaySellExecuted(
            intent.strategyId,
            intent.sessionKey,
            intent.relayOrderId,
            intent.token,
            intent.fundingChainId,
            intent.sellQuantity,
            provisionalCost
        );
    }

    function _validateCreditUsdcReturn(
        CreditUsdcReturnIntent calldata intent,
        bytes calldata sessionSignature
    ) internal returns (address vault) {
        if (intent.relayOrderId == bytes32(0)) {
            revert InvalidIntent();
        }
        _validateSignedSwap(
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            CreditUsdcReturnIntentHash.digest(_creditUsdcReturnPayload(intent), _domainSeparator()),
            sessionSignature
        );
        _consumeRelayReceipt(intent.strategyId, intent.relayOrderId, RelayAction.UsdcReturn);

        if (intent.platformFeeUsdc > intent.usdcReceived) revert InvalidIntent();
        if (intent.platformFeeUsdc == 0) {
            if (intent.feeRecipient != address(0)) revert InvalidIntent();
        } else if (intent.feeRecipient == address(0)) {
            revert InvalidIntent();
        }

        Layout storage $ = _layout();
        vault = _requireStrategyVault($, intent.strategyId);
        if (intent.usdcReceived > _vaultSurplus($, intent.strategyId, usdcToken)) {
            revert InsufficientVaultSurplus();
        }

        Session storage session = $.sessions[intent.strategyId][intent.sessionKey];
        uint256 costReleased = _normalizeUsdc(uint256(intent.destCostReleasedUsdc));
        if (costReleased == 0 || costReleased > session.deployedUsdc) revert InvalidIntent();
    }

    function _applyCreditUsdcReturnAccounting(CreditUsdcReturnIntent calldata intent, address vault)
        internal
        returns (int256 realizedPnlUsdc)
    {
        uint256 netReturn = intent.usdcReceived - intent.platformFeeUsdc;
        if (intent.platformFeeUsdc > 0) {
            _asVault(vault).transferToken(usdcToken, intent.feeRecipient, intent.platformFeeUsdc);
            emit PlatformFeeCharged(
                intent.strategyId, intent.sessionKey, intent.feeRecipient, intent.platformFeeUsdc
            );
        }
        if (netReturn > 0) {
            _asVault(vault).transferToken(usdcToken, address(this), netReturn);
        }

        Session storage session = _layout().sessions[intent.strategyId][intent.sessionKey];
        uint256 costReleased = _normalizeUsdc(uint256(intent.destCostReleasedUsdc));
        uint256 netUsdc = _normalizeUsdc(netReturn);
        session.deployedUsdc = uint128(uint256(session.deployedUsdc) - costReleased);
        if (netUsdc >= costReleased) {
            uint256 profit = netUsdc - costReleased;
            uint256 headroom = uint256(session.limitUsdc) - uint256(session.capacityUsdc);
            if (profit > headroom) profit = headroom;
            session.capacityUsdc = uint128(uint256(session.capacityUsdc) + profit);
            realizedPnlUsdc = int256(profit);
        } else {
            uint256 loss = costReleased - netUsdc;
            uint256 nextCapacity = uint256(session.capacityUsdc);
            if (loss > nextCapacity) nextCapacity = 0;
            else nextCapacity -= loss;
            session.capacityUsdc = uint128(nextCapacity);
            realizedPnlUsdc = -int256(loss);
        }
    }

    function _finalizeCreditUsdcReturn(
        CreditUsdcReturnIntent calldata intent,
        int256 realizedPnlUsdc
    ) internal {
        Session storage session = _layout().sessions[intent.strategyId][intent.sessionKey];
        session.nonce += 1;
        emit UsdcReturnCredited(
            intent.strategyId,
            intent.sessionKey,
            intent.relayOrderId,
            intent.usdcReceived,
            intent.destQuantityReleased,
            intent.destCostReleasedUsdc,
            realizedPnlUsdc
        );
    }

    function _validateWalletRelaySwap(
        WalletRelaySwapIntent calldata intent,
        address relayTarget,
        bytes calldata relayCalldata,
        uint256 relayValue,
        bytes calldata ownerSignature
    ) internal returns (uint256 relayAmount) {
        if (block.timestamp > intent.deadline) revert IntentExpired();
        Layout storage $ = _layout();
        if ($.walletRelayNonce != intent.nonce) revert NonceMismatch();
        if (intent.relayOrderId == bytes32(0)) revert InvalidIntent();
        if ($.walletRelayOrderConsumed[intent.relayOrderId]) revert RelayReceiptConsumed();

        bytes32 relayCallHash = keccak256(abi.encode(relayTarget, relayValue, relayCalldata));
        if (relayCallHash != intent.relayCalldataHash) revert InvalidIntent();
        _validateRelayTarget(relayTarget);

        bytes32 digest =
            WalletRelaySwapIntentHash.digest(_walletRelaySwapPayload(intent), _domainSeparator());
        if (_recover(digest, ownerSignature) != address(this)) revert InvalidSignature();

        if (intent.sellToken == address(0)) {
            if (address(this).balance < intent.sellAmount) revert InsufficientInventory();
        } else if (IERC20Extended(intent.sellToken).balanceOf(address(this)) < intent.sellAmount) {
            revert InsufficientInventory();
        }

        relayAmount = intent.sellAmount;
        if (intent.sellToken == usdcToken) {
            _chargeWalletPlatformFee(intent);
            relayAmount = intent.sellAmount - intent.platformFeeUsdc;
        } else if (intent.platformFeeUsdc != 0 || intent.feeRecipient != address(0)) {
            revert InvalidIntent();
        }
    }

    function _finalizeWalletRelaySwap(WalletRelaySwapIntent calldata intent) internal {
        Layout storage $ = _layout();
        $.walletRelayOrderConsumed[intent.relayOrderId] = true;
        $.walletRelayNonce += 1;
        emit WalletRelaySwapExecuted(
            intent.nonce,
            intent.sellToken,
            intent.sellAmount,
            intent.destRecipient,
            intent.relayOrderId
        );
    }

    function _validateV2Swap(
        SwapBundleIntentV2 calldata intent,
        bytes memory strategyCalldata,
        bytes memory gasCalldata,
        bytes calldata sessionSignature
    ) internal view {
        _validateV2Funding(intent, gasCalldata);

        uint256 routerSellAmount = intent.gasFundingMode == GasFundingMode.NATIVE_OUTPUT
            ? intent.strategySellAmount + intent.gasTopUpUsdc
            : intent.strategySellAmount;
        _validateRouterCalldata(
            strategyCalldata, intent.strategyRouterCalldataHash, intent.sellToken, routerSellAmount
        );
        _validateSignedSwap(
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            SwapBundleIntentV2Hash.digest(_bundleV2Payload(intent), _domainSeparator()),
            sessionSignature
        );
    }

    function _executeV2Bundle(
        Layout storage $,
        SwapBundleIntentV2 calldata intent,
        bytes memory strategyRouterCalldata,
        bytes memory gasRouterCalldata
    ) internal {
        Session storage session = $.sessions[intent.strategyId][intent.sessionKey];
        bool isBuy = intent.sellToken == usdcToken && intent.buyToken != usdcToken;
        bool isSell = intent.buyToken == usdcToken && intent.sellToken != usdcToken;
        if (!isBuy && !isSell) revert InvalidIntent();

        uint256 overheadUsdc =
            _normalizeUsdc(intent.platformFeeUsdc) + _normalizeUsdc(intent.gasTopUpUsdc);
        if (isBuy) {
            uint256 deployable = uint256(session.capacityUsdc) - uint256(session.deployedUsdc);
            if (_normalizeUsdc(intent.strategySellAmount) + overheadUsdc > deployable) {
                revert SpendLimitExceeded();
            }
        }

        (uint256 sellAmount, uint256 buyAmount) = _executeRouterSwap(
            intent.sellToken,
            intent.buyToken,
            intent.strategySellAmount,
            strategyRouterCalldata,
            $.assets[intent.strategyId][intent.sellToken].quantity
        );
        if (buyAmount < intent.minStrategyBuyAmount) revert SlippageExceeded();

        int256 realizedPnlUsdc = _applySwapAccounting(
            $, intent.strategyId, session, intent.sellToken, intent.buyToken, sellAmount, buyAmount
        );
        _deductOverhead(session, overheadUsdc);
        _chargePlatformFeeV2(intent);
        _fundSeparateGas(intent, gasRouterCalldata);

        session.nonce += 1;
        emit SwapExecuted(
            intent.strategyId,
            intent.sessionKey,
            intent.sellToken,
            intent.buyToken,
            sellAmount,
            buyAmount,
            realizedPnlUsdc
        );
    }

    function _executeNativeOutputBundle(
        Layout storage $,
        SwapBundleIntentV2 calldata intent,
        bytes memory strategyRouterCalldata
    ) internal {
        if (intent.sellToken != usdcToken || intent.buyToken != address(0)) {
            revert InvalidIntent();
        }
        Session storage session = $.sessions[intent.strategyId][intent.sessionKey];
        uint256 overheadUsdc =
            _normalizeUsdc(intent.platformFeeUsdc) + _normalizeUsdc(intent.gasTopUpUsdc);
        uint256 strategyCostUsdc = _normalizeUsdc(intent.strategySellAmount);
        uint256 deployable = uint256(session.capacityUsdc) - uint256(session.deployedUsdc);
        if (strategyCostUsdc + overheadUsdc > deployable) revert SpendLimitExceeded();

        _deductOverhead(session, overheadUsdc);
        _chargePlatformFeeV2(intent);

        uint256 totalRouterInput = intent.strategySellAmount + intent.gasTopUpUsdc;
        (uint256 totalSold, uint256 grossNativeOut) =
            _executeRouterSwap(usdcToken, address(0), totalRouterInput, strategyRouterCalldata, 0);
        if (totalSold != totalRouterInput) revert InvalidIntent();
        if (grossNativeOut < intent.minStrategyBuyAmount + intent.gasTopUpNative) {
            revert SlippageExceeded();
        }

        uint256 netStrategyNative = grossNativeOut - intent.gasTopUpNative;
        AssetRecord storage nativeAsset = $.assets[intent.strategyId][address(0)];
        _executeBuy(session, nativeAsset, netStrategyNative, strategyCostUsdc);
        _transferGasCredit(intent, intent.gasTopUpNative);

        session.nonce += 1;
        emit SwapExecuted(
            intent.strategyId,
            intent.sessionKey,
            usdcToken,
            address(0),
            intent.strategySellAmount,
            netStrategyNative,
            0
        );
    }

    function _asVault(address vault) internal pure returns (StrategyVault) {
        return StrategyVault(payable(vault));
    }

    function _ensureStrategyVault(Layout storage $, bytes32 strategyId) internal {
        if ($.strategyVault[strategyId] != address(0)) return;
        address vault =
            address(new StrategyVault{salt: keccak256(abi.encode(strategyId))}(address(this)));
        $.strategyVault[strategyId] = vault;
        emit StrategyVaultDeployed(strategyId, vault);
    }

    function _requireStrategyVault(Layout storage $, bytes32 strategyId)
        internal
        view
        returns (address vault)
    {
        vault = $.strategyVault[strategyId];
        if (vault == address(0)) revert InvalidIntent();
    }

    function _validateRelayDepositRecipients(RelayDepositIntent calldata intent, address vault)
        internal
        view
    {
        if (intent.refundVault != vault) revert InvalidIntent();
        if (intent.destChainId != block.chainid) {
            if (intent.destRecipient == address(0)) revert InvalidIntent();
            return;
        }
        if (intent.destRecipient != vault) revert InvalidIntent();
    }

    function _lockUsdcRelayDeposit(
        Layout storage $,
        Session storage session,
        address vault,
        RelayDepositIntent calldata intent
    ) internal {
        uint256 overheadUsdc = _normalizeUsdc(intent.platformFeeUsdc);
        uint256 deployCost = _normalizeUsdc(intent.originAmount);
        uint256 deployable = uint256(session.capacityUsdc) - uint256(session.deployedUsdc);
        if (deployCost + overheadUsdc > deployable) revert SpendLimitExceeded();
        _deductOverhead(session, overheadUsdc);
        _fundVaultAndChargeRelayFee(vault, intent);
        session.deployedUsdc = uint128(uint256(session.deployedUsdc) + deployCost);
        $.pendingDeposits[intent.relayOrderId] = PendingDeposit({
            strategyId: intent.strategyId,
            lockedCostUsdc: uint128(deployCost),
            refundVault: intent.refundVault,
            originAmount: intent.originAmount,
            originToken: intent.originToken,
            fundingChainId: intent.fundingChainId,
            exists: true
        });
    }

    function _lockRemoteInventoryRelayDeposit(Layout storage $, RelayDepositIntent calldata intent)
        internal
    {
        if (intent.platformFeeUsdc != 0 || intent.feeRecipient != address(0)) {
            revert InvalidIntent();
        }
        uint128 costPortion = _consumeRemoteInventory(
            $, intent.strategyId, intent.originToken, intent.fundingChainId, intent.originAmount
        );
        _decreaseVaultAccounted($, intent.strategyId, intent.originToken, intent.originAmount);
        $.pendingDeposits[intent.relayOrderId] = PendingDeposit({
            strategyId: intent.strategyId,
            lockedCostUsdc: costPortion,
            refundVault: intent.refundVault,
            originAmount: intent.originAmount,
            originToken: intent.originToken,
            fundingChainId: intent.fundingChainId,
            exists: true
        });
    }

    function _fundVaultAndChargeRelayFee(address vault, RelayDepositIntent calldata intent)
        internal
    {
        if (intent.platformFeeUsdc > 0) {
            if (intent.feeRecipient == address(0)) revert InvalidIntent();
            if (IERC20Extended(usdcToken).balanceOf(address(this)) < intent.originAmount) {
                revert InsufficientInventory();
            }
            if (!IERC20Extended(usdcToken).transfer(vault, intent.originAmount)) {
                revert CallFailed("");
            }
            _asVault(vault).transferToken(usdcToken, intent.feeRecipient, intent.platformFeeUsdc);
            emit PlatformFeeCharged(
                intent.strategyId, intent.sessionKey, intent.feeRecipient, intent.platformFeeUsdc
            );
        } else {
            if (intent.feeRecipient != address(0)) revert InvalidIntent();
            _transferToVault(vault, usdcToken, intent.originAmount);
        }
    }

    function _transferToVault(address vault, address token, uint256 amount) internal {
        if (token == address(0)) {
            uint256 nativeVaultBal = vault.balance;
            if (nativeVaultBal >= amount) return;
            uint256 nativeDeficit = amount - nativeVaultBal;
            if (address(this).balance < nativeDeficit) revert InsufficientInventory();
            (bool sent,) = payable(vault).call{value: nativeDeficit}("");
            if (!sent) revert CallFailed("");
            return;
        }
        uint256 tokenVaultBal = IERC20Extended(token).balanceOf(vault);
        if (tokenVaultBal >= amount) return;
        uint256 tokenDeficit = amount - tokenVaultBal;
        if (IERC20Extended(token).balanceOf(address(this)) < tokenDeficit) {
            revert InsufficientInventory();
        }
        if (!IERC20Extended(token).transfer(vault, tokenDeficit)) revert CallFailed("");
    }

    function _vaultTokenBalance(address vault, address token) internal view returns (uint256) {
        if (token == address(0)) return vault.balance;
        return IERC20Extended(token).balanceOf(vault);
    }

    function _vaultSurplus(Layout storage $, bytes32 strategyId, address token)
        internal
        view
        returns (uint256)
    {
        address vault = $.strategyVault[strategyId];
        if (vault == address(0)) return 0;
        uint256 balance = _vaultTokenBalance(vault, token);
        uint256 accounted = $.vaultAccountedBalance[strategyId][token];
        return balance > accounted ? balance - accounted : 0;
    }

    function _increaseVaultAccounted(
        Layout storage $,
        bytes32 strategyId,
        address token,
        uint256 amount
    ) internal {
        $.vaultAccountedBalance[strategyId][token] += amount;
    }

    function _decreaseVaultAccounted(
        Layout storage $,
        bytes32 strategyId,
        address token,
        uint256 amount
    ) internal {
        uint256 accounted = $.vaultAccountedBalance[strategyId][token];
        if (amount > accounted) revert InvalidIntent();
        $.vaultAccountedBalance[strategyId][token] = accounted - amount;
    }

    function _consumeRemoteInventory(
        Layout storage $,
        bytes32 strategyId,
        address token,
        uint256 fundingChainId,
        uint256 amount
    ) internal returns (uint128 costPortion) {
        RemoteAssetRecord storage remote = $.remoteAssets[strategyId][token][fundingChainId];
        if (remote.quantity < amount) revert InsufficientInventory();
        costPortion = remote.quantity == amount
            ? remote.costUsdc
            : uint128((uint256(remote.costUsdc) * amount) / remote.quantity);
        remote.quantity -= amount;
        remote.costUsdc = uint128(uint256(remote.costUsdc) - uint256(costPortion));
    }

    function _consumeRelayReceipt(bytes32 strategyId, bytes32 relayOrderId, RelayAction action)
        internal
    {
        Layout storage $ = _layout();
        if ($.consumedRelayReceipts[strategyId][relayOrderId][uint8(action)]) {
            revert RelayReceiptConsumed();
        }
        $.consumedRelayReceipts[strategyId][relayOrderId][uint8(action)] = true;
    }

    function _validateRelayDecimals(uint8 originDecimals, uint8 destDecimals) internal pure {
        if (originDecimals > 18 || destDecimals > 18) revert InvalidIntent();
    }

    function _executeVaultRelayCall(
        address vault,
        address originToken,
        uint256 originAmount,
        address relayTarget,
        bytes calldata relayCalldata,
        uint256 relayValue
    ) internal {
        if (originToken == address(0)) {
            if (relayValue != originAmount) revert RouterFieldsMismatch();
            _transferToVault(vault, address(0), originAmount);
            _asVault(vault).callWithValue(relayTarget, relayCalldata, originAmount);
            return;
        }
        if (relayValue != 0) revert RouterFieldsMismatch();
        _transferToVault(vault, originToken, originAmount);
        _asVault(vault).approveToken(originToken, relayTarget, originAmount);
        _asVault(vault).callExternal(relayTarget, relayCalldata);
        _asVault(vault).approveToken(originToken, relayTarget, 0);
    }

    function _validateSignedSwap(
        bytes32 strategyId,
        address sessionKey,
        uint256 nonce,
        uint256 deadline,
        bytes32 digest,
        bytes calldata sessionSignature
    ) internal view {
        if (block.timestamp > deadline) revert IntentExpired();
        Session storage session = _layout().sessions[strategyId][sessionKey];
        _assertActive(session);
        if (session.nonce != nonce) revert NonceMismatch();
        if (_recover(digest, sessionSignature) != sessionKey) revert InvalidSignature();
    }

    function _revoke(bytes32 strategyId, address key, Session storage session) internal {
        session.revoked = true;
        session.nonce += 1;
        emit SessionRevocation(strategyId, key, session.nonce);
    }

    function _applySwapAccounting(
        Layout storage $,
        bytes32 strategyId,
        Session storage session,
        address sellToken,
        address buyToken,
        uint256 sellAmount,
        uint256 buyAmount
    ) internal returns (int256 realizedPnlUsdc) {
        address configuredUsdc = usdcToken;
        AssetRecord storage sellAsset = $.assets[strategyId][sellToken];
        AssetRecord storage buyAsset = $.assets[strategyId][buyToken];

        if (sellToken == configuredUsdc && buyToken != configuredUsdc) {
            _executeBuy(session, buyAsset, buyAmount, _normalizeUsdc(sellAmount));
            return 0;
        }

        if (buyToken == configuredUsdc && sellToken != configuredUsdc) {
            return _executeSell(session, sellAsset, sellAmount, _normalizeUsdc(buyAmount));
        }

        revert InvalidIntent();
    }

    function _executeBuy(
        Session storage session,
        AssetRecord storage buyAsset,
        uint256 buyAmount,
        uint256 usdcSpent
    ) internal {
        uint256 deployable =
            uint256(session.capacityUsdc) - uint256(session.deployedUsdc);
        if (usdcSpent > deployable) revert SpendLimitExceeded();

        session.deployedUsdc = uint128(uint256(session.deployedUsdc) + usdcSpent);
        buyAsset.quantity += buyAmount;
        buyAsset.costUsdc = uint128(uint256(buyAsset.costUsdc) + usdcSpent);
    }

    function _executeSell(
        Session storage session,
        AssetRecord storage sellAsset,
        uint256 sellAmount,
        uint256 usdcReceived
    ) internal returns (int256 realizedPnlUsdc) {
        if (sellAsset.quantity < sellAmount) revert InsufficientInventory();

        uint256 costReleased = sellAsset.quantity == sellAmount
            ? uint256(sellAsset.costUsdc)
            : (uint256(sellAsset.costUsdc) * sellAmount) / sellAsset.quantity;

        sellAsset.quantity -= sellAmount;
        sellAsset.costUsdc = uint128(uint256(sellAsset.costUsdc) - costReleased);
        session.deployedUsdc = uint128(uint256(session.deployedUsdc) - costReleased);

        if (usdcReceived >= costReleased) {
            uint256 profit = usdcReceived - costReleased;
            uint256 headroom = uint256(session.limitUsdc) - uint256(session.capacityUsdc);
            if (profit > headroom) {
                profit = headroom;
            }
            session.capacityUsdc = uint128(uint256(session.capacityUsdc) + profit);
            realizedPnlUsdc = int256(profit);
        } else {
            uint256 loss = costReleased - usdcReceived;
            uint256 nextCapacity = uint256(session.capacityUsdc);
            if (loss > nextCapacity) {
                nextCapacity = 0;
            } else {
                nextCapacity -= loss;
            }
            session.capacityUsdc = uint128(nextCapacity);
            realizedPnlUsdc = -int256(loss);
        }
    }

    function _normalizeUsdc(uint256 amount) internal view returns (uint256) {
        uint8 decimals = usdcDecimals;
        if (decimals == 6) {
            return amount;
        }
        if (decimals > 6) {
            return amount / (10 ** (decimals - 6));
        }
        return amount * (10 ** (6 - decimals));
    }

    function _routerSelector(bytes memory routerCalldata) internal pure returns (bytes4 selector) {
        assembly {
            selector := mload(add(routerCalldata, 32))
        }
    }

    function _validateRouterCalldata(
        bytes memory routerCalldata,
        bytes32 routerCalldataHash,
        address sellToken,
        uint256 maxSellAmount
    ) internal pure {
        if (routerCalldata.length < 4) revert SelectorNotAllowed();
        if (_routerSelector(routerCalldata) != EXEC_SELECTOR) revert SelectorNotAllowed();
        if (keccak256(routerCalldata) != routerCalldataHash) revert InvalidIntent();
        (
            address operator,
            address calldataSellToken,
            uint256 calldataSellAmount,
            address target,
            bytes memory targetCalldata
        ) = abi.decode(_routerArgs(routerCalldata), (address, address, uint256, address, bytes));
        if (
            !_routerTokenMatches(sellToken, calldataSellToken) || calldataSellAmount != maxSellAmount
                || operator == address(0) || target == address(0) || operator != target
        ) revert RouterFieldsMismatch();
        if (targetCalldata.length < 4) revert RouterFieldsMismatch();
    }

    function _validateV2Funding(SwapBundleIntentV2 calldata intent, bytes memory gasRouterCalldata)
        internal
        view
    {
        if (intent.gasFundingMode == GasFundingMode.CREDIT_ONLY) {
            if (
                intent.gasTopUpUsdc != 0 || intent.gasTopUpNative != 0
                    || intent.gasRecipient != address(0)
                    || intent.gasRouterCalldataHash != bytes32(0) || gasRouterCalldata.length != 0
            ) revert InvalidIntent();
            return;
        }

        if (
            intent.gasTopUpUsdc == 0 || intent.gasTopUpNative == 0
                || intent.gasRecipient != msg.sender
        ) {
            revert InvalidIntent();
        }

        if (intent.gasFundingMode == GasFundingMode.SEPARATE_TOPUP) {
            _validateRouterCalldata(
                gasRouterCalldata, intent.gasRouterCalldataHash, usdcToken, intent.gasTopUpUsdc
            );
            return;
        }

        if (
            intent.sellToken != usdcToken || intent.buyToken != address(0)
                || intent.gasRouterCalldataHash != bytes32(0) || gasRouterCalldata.length != 0
        ) revert InvalidIntent();
    }

    function _executeRouterSwap(
        address sellToken,
        address buyToken,
        uint256 maxSellAmount,
        bytes memory routerCalldata
    ) internal returns (uint256 sellAmount, uint256 buyAmount) {
        uint256 sellBefore = sellToken == address(0)
            ? address(this).balance
            : IERC20Extended(sellToken).balanceOf(address(this));
        uint256 buyBefore = buyToken == address(0)
            ? address(this).balance
            : IERC20Extended(buyToken).balanceOf(address(this));

        bool isNativeSell = sellToken == address(0);
        if (!isNativeSell) {
            _forceApprove(sellToken, ALLOWANCE_HOLDER, maxSellAmount);
        }
        (bool ok, bytes memory result) = isNativeSell
            ? ALLOWANCE_HOLDER.call{value: maxSellAmount}(routerCalldata)
            : ALLOWANCE_HOLDER.call(routerCalldata);
        if (!ok) revert CallFailed(result);
        if (!isNativeSell) {
            _forceApprove(sellToken, ALLOWANCE_HOLDER, 0);
        }

        uint256 sellAfter = sellToken == address(0)
            ? address(this).balance
            : IERC20Extended(sellToken).balanceOf(address(this));
        uint256 buyAfter = buyToken == address(0)
            ? address(this).balance
            : IERC20Extended(buyToken).balanceOf(address(this));

        sellAmount = sellBefore - sellAfter;
        buyAmount = buyAfter - buyBefore;
        if (sellAmount == 0 || buyAmount == 0) revert InvalidIntent();
        if (sellAmount > maxSellAmount) revert SlippageExceeded();
    }

    function _executeRouterSwap(
        address sellToken,
        address buyToken,
        uint256 maxSellAmount,
        bytes memory routerCalldata,
        uint256 recordedInventory
    ) internal returns (uint256 sellAmount, uint256 buyAmount) {
        if (sellToken != usdcToken && recordedInventory < maxSellAmount) {
            revert InsufficientInventory();
        }
        return _executeRouterSwap(sellToken, buyToken, maxSellAmount, routerCalldata);
    }

    /// @dev A native sell has no allowance for AllowanceHolder to hold, so 0x leaves the token
    ///      slot empty even though the settler actions name the ETH sentinel. The forwarded value
    ///      and the balance delta both bind the spend, so either shape is safe here.
    function _routerTokenMatches(address intentToken, address calldataToken)
        internal
        pure
        returns (bool)
    {
        if (intentToken == address(0)) {
            return calldataToken == address(0) || calldataToken == ZEROX_NATIVE_TOKEN;
        }
        return calldataToken == intentToken;
    }

    function _routerArgs(bytes memory routerCalldata) internal pure returns (bytes memory) {
        bytes memory args = new bytes(routerCalldata.length - 4);
        for (uint256 i = 0; i < args.length; ++i) {
            args[i] = routerCalldata[i + 4];
        }
        return args;
    }

    function _chargePlatformFeeV2(SwapBundleIntentV2 calldata intent) internal {
        if (intent.platformFeeUsdc == 0) {
            if (intent.feeRecipient != address(0)) revert InvalidIntent();
            return;
        }
        if (intent.feeRecipient == address(0)) revert InvalidIntent();
        if (!IERC20Extended(usdcToken).transfer(intent.feeRecipient, intent.platformFeeUsdc)) {
            revert CallFailed("");
        }
        emit PlatformFeeCharged(
            intent.strategyId, intent.sessionKey, intent.feeRecipient, intent.platformFeeUsdc
        );
    }

    function _deductOverhead(Session storage session, uint256 overheadUsdc) internal {
        if (overheadUsdc == 0) return;
        if (overheadUsdc > session.capacityUsdc) revert SpendLimitExceeded();
        session.capacityUsdc = uint128(uint256(session.capacityUsdc) - overheadUsdc);
    }

    function _fundSeparateGas(SwapBundleIntentV2 calldata intent, bytes memory gasRouterCalldata)
        internal
    {
        if (intent.gasFundingMode != GasFundingMode.SEPARATE_TOPUP) return;
        (, uint256 nativeOut) =
            _executeRouterSwap(usdcToken, address(0), intent.gasTopUpUsdc, gasRouterCalldata);
        if (nativeOut < intent.gasTopUpNative) revert SlippageExceeded();
        _transferGasCredit(intent, nativeOut);
    }

    function _transferGasCredit(SwapBundleIntentV2 calldata intent, uint256 nativeAmount) internal {
        if (intent.gasRecipient != msg.sender) revert InvalidIntent();
        (bool sent,) = msg.sender.call{value: nativeAmount}("");
        if (!sent) revert CallFailed("");
        emit GasCreditFunded(
            intent.strategyId,
            intent.sessionKey,
            intent.gasRecipient,
            intent.gasFundingMode,
            intent.gasTopUpUsdc,
            nativeAmount
        );
    }

    function _forceApprove(address token, address spender, uint256 amount) internal {
        if (_tryApprove(token, spender, amount)) return;
        if (!_tryApprove(token, spender, 0) || !_tryApprove(token, spender, amount)) {
            revert ApprovalFailed(token, amount);
        }
    }

    function _tryApprove(address token, address spender, uint256 amount) internal returns (bool) {
        (bool ok, bytes memory result) =
            token.call(abi.encodeCall(IERC20Extended.approve, (spender, amount)));
        if (!ok) return false;
        if (result.length == 0) return true;
        if (result.length < 32) return false;
        uint256 returned;
        assembly {
            returned := mload(add(result, 32))
        }
        return returned == 1;
    }

    function _assertActive(Session storage session) internal view {
        if (!session.exists) revert SessionUnknown();
        if (session.revoked) revert SessionRevoked();
        if (session.expiresAt <= block.timestamp) revert SessionExpired();
    }

    function _indexStrategy(Layout storage $, bytes32 strategyId) internal {
        if ($.strategyIndex[strategyId] != 0) return;
        $.strategyIds.push(strategyId);
        $.strategyIndex[strategyId] = $.strategyIds.length;
    }

    function _indexSession(Layout storage $, bytes32 strategyId, address key) internal {
        if ($.sessionKeyIndex[strategyId][key] != 0) return;
        $.sessionKeys[strategyId].push(key);
        $.sessionKeyIndex[strategyId][key] = $.sessionKeys[strategyId].length;
    }

    function _bundleV2Payload(SwapBundleIntentV2 calldata intent)
        internal
        pure
        returns (SwapBundleIntentV2Payload memory payload)
    {
        payload = SwapBundleIntentV2Payload({
            strategyId: intent.strategyId,
            sessionKey: intent.sessionKey,
            nonce: intent.nonce,
            deadline: intent.deadline,
            sellToken: intent.sellToken,
            buyToken: intent.buyToken,
            strategySellAmount: intent.strategySellAmount,
            minStrategyBuyAmount: intent.minStrategyBuyAmount,
            strategyRouterCalldataHash: intent.strategyRouterCalldataHash,
            platformFeeUsdc: intent.platformFeeUsdc,
            feeRecipient: intent.feeRecipient,
            gasFundingMode: uint8(intent.gasFundingMode),
            gasTopUpUsdc: intent.gasTopUpUsdc,
            gasTopUpNative: intent.gasTopUpNative,
            gasRecipient: intent.gasRecipient,
            gasRouterCalldataHash: intent.gasRouterCalldataHash
        });
    }

    function _revokeIntentDigest(RevokeIntent calldata intent) internal view returns (bytes32) {
        return keccak256(
            abi.encodePacked(
                "\x19\x01",
                _domainSeparator(),
                keccak256(
                    abi.encode(
                        REVOKE_INTENT_TYPEHASH,
                        intent.strategyId,
                        intent.sessionKey,
                        intent.nonce,
                        intent.deadline
                    )
                )
            )
        );
    }

    function _relayDepositPayload(RelayDepositIntent calldata intent)
        internal
        pure
        returns (RelayDepositIntentPayload memory payload)
    {
        payload = RelayDepositIntentPayload({
            strategyId: intent.strategyId,
            sessionKey: intent.sessionKey,
            nonce: intent.nonce,
            deadline: intent.deadline,
            relayOrderId: intent.relayOrderId,
            fundingChainId: intent.fundingChainId,
            originToken: intent.originToken,
            originTokenDecimals: intent.originTokenDecimals,
            destToken: intent.destToken,
            destTokenDecimals: intent.destTokenDecimals,
            originAmount: intent.originAmount,
            destChainId: intent.destChainId,
            minDestAmount: intent.minDestAmount,
            destRecipient: intent.destRecipient,
            refundVault: intent.refundVault,
            relayCalldataHash: intent.relayCalldataHash,
            platformFeeUsdc: intent.platformFeeUsdc,
            feeRecipient: intent.feeRecipient
        });
    }

    function _creditRelayAssetPayload(CreditRelayAssetIntent calldata intent)
        internal
        pure
        returns (CreditRelayAssetIntentPayload memory payload)
    {
        payload = CreditRelayAssetIntentPayload({
            strategyId: intent.strategyId,
            sessionKey: intent.sessionKey,
            nonce: intent.nonce,
            deadline: intent.deadline,
            relayOrderId: intent.relayOrderId,
            token: intent.token,
            fundingChainId: intent.fundingChainId,
            creditQuantity: intent.creditQuantity,
            costUsdc: intent.costUsdc
        });
    }

    function _remoteRelaySellPayload(RemoteRelaySellIntent calldata intent)
        internal
        pure
        returns (RemoteRelaySellIntentPayload memory payload)
    {
        payload = RemoteRelaySellIntentPayload({
            strategyId: intent.strategyId,
            sessionKey: intent.sessionKey,
            nonce: intent.nonce,
            deadline: intent.deadline,
            relayOrderId: intent.relayOrderId,
            token: intent.token,
            fundingChainId: intent.fundingChainId,
            sellQuantity: intent.sellQuantity,
            minReturnUsdc: intent.minReturnUsdc,
            relayCalldataHash: intent.relayCalldataHash
        });
    }

    function _creditUsdcReturnPayload(CreditUsdcReturnIntent calldata intent)
        internal
        pure
        returns (CreditUsdcReturnIntentPayload memory payload)
    {
        payload = CreditUsdcReturnIntentPayload({
            strategyId: intent.strategyId,
            sessionKey: intent.sessionKey,
            nonce: intent.nonce,
            deadline: intent.deadline,
            relayOrderId: intent.relayOrderId,
            fundingChainId: intent.fundingChainId,
            usdcReceived: intent.usdcReceived,
            destQuantityReleased: intent.destQuantityReleased,
            destCostReleasedUsdc: intent.destCostReleasedUsdc,
            platformFeeUsdc: intent.platformFeeUsdc,
            feeRecipient: intent.feeRecipient
        });
    }

    function _releaseRelayDepositPayload(ReleaseRelayDepositIntent calldata intent)
        internal
        pure
        returns (ReleaseRelayDepositIntentPayload memory payload)
    {
        payload = ReleaseRelayDepositIntentPayload({
            strategyId: intent.strategyId,
            sessionKey: intent.sessionKey,
            nonce: intent.nonce,
            deadline: intent.deadline,
            relayOrderId: intent.relayOrderId,
            token: intent.token,
            fundingChainId: intent.fundingChainId,
            refundQuantity: intent.refundQuantity,
            refundCostUsdc: intent.refundCostUsdc
        });
    }

    function _restoreRemoteRelayAssetPayload(RestoreRemoteRelayAssetIntent calldata intent)
        internal
        pure
        returns (RestoreRemoteRelayAssetIntentPayload memory payload)
    {
        payload = RestoreRemoteRelayAssetIntentPayload({
            strategyId: intent.strategyId,
            sessionKey: intent.sessionKey,
            nonce: intent.nonce,
            deadline: intent.deadline,
            relayOrderId: intent.relayOrderId,
            token: intent.token,
            fundingChainId: intent.fundingChainId,
            restoreQuantity: intent.restoreQuantity,
            restoreCostUsdc: intent.restoreCostUsdc
        });
    }

    function _relayGasTopUpPayload(RelayGasTopUpIntent calldata intent)
        internal
        pure
        returns (RelayGasTopUpIntentPayload memory payload)
    {
        payload = RelayGasTopUpIntentPayload({
            strategyId: intent.strategyId,
            sessionKey: intent.sessionKey,
            nonce: intent.nonce,
            deadline: intent.deadline,
            relayOrderId: intent.relayOrderId,
            overheadUsdc: intent.overheadUsdc,
            gasRecipient: intent.gasRecipient,
            relayCalldataHash: intent.relayCalldataHash
        });
    }

    function _releaseRelayGasTopUpPayload(ReleaseRelayGasTopUpIntent calldata intent)
        internal
        pure
        returns (ReleaseRelayGasTopUpIntentPayload memory payload)
    {
        payload = ReleaseRelayGasTopUpIntentPayload({
            strategyId: intent.strategyId,
            sessionKey: intent.sessionKey,
            nonce: intent.nonce,
            deadline: intent.deadline,
            relayOrderId: intent.relayOrderId,
            refundUsdc: intent.refundUsdc
        });
    }

    function _validateRelayTarget(address target) internal pure {
        if (
            target == RELAY_ERC20_ROUTER || target == RELAY_APPROVAL_PROXY
                || target == RELAY_DEPOSITORY || target == RELAY_DEPOSITORY_ALT
                || target == RELAY_ERC20_ROUTER_ALT || target == RELAY_APPROVAL_PROXY_ALT
                || target == RELAY_RECEIVER || target == RELAY_MULTICALLER
        ) return;
        revert SelectorNotAllowed();
    }

    function _executeRelayCall(
        address originToken,
        uint256 originAmount,
        address relayTarget,
        bytes calldata relayCalldata,
        uint256 relayValue
    ) internal {
        if (originToken == address(0)) {
            if (relayValue != originAmount) revert RouterFieldsMismatch();
            (bool ok, bytes memory result) = relayTarget.call{value: relayValue}(relayCalldata);
            if (!ok) revert CallFailed(result);
            return;
        }
        if (relayValue != 0) revert RouterFieldsMismatch();
        _forceApprove(originToken, relayTarget, originAmount);
        (bool okErc20, bytes memory resultErc20) = relayTarget.call(relayCalldata);
        if (!okErc20) revert CallFailed(resultErc20);
        _forceApprove(originToken, relayTarget, 0);
    }

    function _walletRelaySwapPayload(WalletRelaySwapIntent calldata intent)
        internal
        pure
        returns (WalletRelaySwapIntentPayload memory payload)
    {
        payload = WalletRelaySwapIntentPayload({
            nonce: intent.nonce,
            deadline: intent.deadline,
            sellToken: intent.sellToken,
            sellAmount: intent.sellAmount,
            destRecipient: intent.destRecipient,
            relayCalldataHash: intent.relayCalldataHash,
            relayOrderId: intent.relayOrderId,
            platformFeeUsdc: intent.platformFeeUsdc,
            feeRecipient: intent.feeRecipient
        });
    }

    function _chargeWalletPlatformFee(WalletRelaySwapIntent calldata intent) internal {
        if (intent.platformFeeUsdc == 0) {
            if (intent.feeRecipient != address(0)) revert InvalidIntent();
            return;
        }
        if (intent.feeRecipient == address(0)) revert InvalidIntent();
        if (intent.platformFeeUsdc > intent.sellAmount) revert InvalidIntent();
        if (!IERC20Extended(usdcToken).transfer(intent.feeRecipient, intent.platformFeeUsdc)) {
            revert CallFailed("");
        }
        emit WalletPlatformFeeCharged(intent.feeRecipient, intent.platformFeeUsdc);
    }

    function _domainSeparator() internal view returns (bytes32) {
        return keccak256(
            abi.encode(
                keccak256(
                    "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
                ),
                keccak256(bytes("PocklessSessionSpend7702")),
                keccak256(bytes("1")),
                block.chainid,
                address(this)
            )
        );
    }

    function _recover(bytes32 digest, bytes calldata signature) internal pure returns (address) {
        if (signature.length != 65) revert InvalidSignature();
        bytes32 r;
        bytes32 s;
        uint8 v;
        assembly {
            r := calldataload(signature.offset)
            s := calldataload(add(signature.offset, 32))
            v := byte(0, calldataload(add(signature.offset, 64)))
        }
        if (v < 27) v += 27;
        address recovered = ecrecover(digest, v, r, s);
        if (recovered == address(0)) revert InvalidSignature();
        return recovered;
    }

    function _layout() internal pure returns (Layout storage $) {
        bytes32 slot = STORAGE_LOCATION;
        assembly {
            $.slot := slot
        }
    }
}
