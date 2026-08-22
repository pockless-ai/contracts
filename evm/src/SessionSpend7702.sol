// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IERC20 {
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
    function transfer(address to, uint256 amount) external returns (bool);
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
    function decimals() external view returns (uint8);
}

struct SwapBundleIntentPayload {
    bytes32 strategyId;
    address sessionKey;
    uint256 nonce;
    uint256 deadline;
    address sellToken;
    address buyToken;
    uint256 maxSellAmount;
    uint256 minBuyAmount;
    bytes32 routerCalldataHash;
    uint256 platformFeeUsdc;
    address feeRecipient;
    uint256 gasSellUsdc;
    uint256 minNativeOut;
    address gasRecipient;
    bytes32 gasRouterCalldataHash;
}

library SwapBundleIntentHash {
    bytes32 internal constant CORE_TYPEHASH = keccak256(
        "SwapBundleCore(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,address sellToken,address buyToken,uint256 maxSellAmount,uint256 minBuyAmount,bytes32 routerCalldataHash)"
    );
    bytes32 internal constant FEES_TYPEHASH = keccak256(
        "SwapBundleFees(uint256 platformFeeUsdc,address feeRecipient,uint256 gasSellUsdc,uint256 minNativeOut,address gasRecipient,bytes32 gasRouterCalldataHash)"
    );
    bytes32 internal constant BUNDLE_TYPEHASH =
        keccak256("SwapBundleIntent(SwapBundleCore core,SwapBundleFees fees)");

    function digest(SwapBundleIntentPayload memory intent, bytes32 domainSeparator)
        internal
        pure
        returns (bytes32)
    {
        bytes32 coreHash = keccak256(
            abi.encode(
                CORE_TYPEHASH,
                intent.strategyId,
                intent.sessionKey,
                intent.nonce,
                intent.deadline,
                intent.sellToken,
                intent.buyToken,
                intent.maxSellAmount,
                intent.minBuyAmount,
                intent.routerCalldataHash
            )
        );
        bytes32 feesHash = keccak256(
            abi.encode(
                FEES_TYPEHASH,
                intent.platformFeeUsdc,
                intent.feeRecipient,
                intent.gasSellUsdc,
                intent.minNativeOut,
                intent.gasRecipient,
                intent.gasRouterCalldataHash
            )
        );
        return keccak256(
            abi.encodePacked(
                "\x19\x01",
                domainSeparator,
                keccak256(abi.encode(BUNDLE_TYPEHASH, coreHash, feesHash))
            )
        );
    }
}

/// @title SessionSpend7702
/// @notice ERC-7702 implementation for per-strategy session keys that may swap
///         through a pinned 0x AllowanceHolder. Storage is ERC-7201 namespaced.
contract SessionSpend7702 {
    /// keccak256(abi.encode(uint256(keccak256("pockless.session.spend7702.v1")) - 1)) & ~bytes32(uint256(0xff))
    bytes32 private constant STORAGE_LOCATION =
        0x3e0c859c46df804f27f96dac030007e24ba5d79c9f807df46118f59ed197e100;

    address public constant ALLOWANCE_HOLDER = 0x0000000000001fF3684f28c67538d4D072C22734;

    bytes4 private constant EXEC_SELECTOR = 0x2213bc0b;

    bytes32 private constant SWAP_INTENT_TYPEHASH = keccak256(
        "SwapIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,address sellToken,address buyToken,uint256 maxSellAmount,uint256 minBuyAmount,bytes32 routerCalldataHash)"
    );
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

    struct SwapIntent {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
        address sellToken;
        address buyToken;
        uint256 maxSellAmount;
        uint256 minBuyAmount;
        bytes32 routerCalldataHash;
    }

    struct SwapBundleIntent {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
        address sellToken;
        address buyToken;
        uint256 maxSellAmount;
        uint256 minBuyAmount;
        bytes32 routerCalldataHash;
        uint256 platformFeeUsdc;
        address feeRecipient;
        uint256 gasSellUsdc;
        uint256 minNativeOut;
        address gasRecipient;
        bytes32 gasRouterCalldataHash;
    }

    struct RevokeIntent {
        bytes32 strategyId;
        address sessionKey;
        uint256 nonce;
        uint256 deadline;
    }

    struct Layout {
        uint256 locked;
        bytes32[] strategyIds;
        mapping(bytes32 => uint256) strategyIndex;
        mapping(bytes32 => address[]) sessionKeys;
        mapping(bytes32 => mapping(address => uint256)) sessionKeyIndex;
        mapping(bytes32 => mapping(address => Session)) sessions;
        mapping(bytes32 => mapping(address => AssetRecord)) assets;
    }

    error NotOwner();
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
    event GasReimbursed(
        bytes32 indexed strategyId,
        address indexed sessionKey,
        address indexed gasRecipient,
        uint256 gasSellUsdc,
        uint256 nativeOut
    );

    modifier onlyOwner() {
        if (msg.sender != address(this)) revert NotOwner();
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
        usdcDecimals = IERC20(usdcToken_).decimals();
    }

    receive() external payable {}

    function grant(bytes32 strategyId, address key, uint256 limitUsdc, uint256 expiresAt)
        external
        onlyOwner
    {
        if (strategyId == bytes32(0)) revert ZeroStrategy();
        if (key == address(0)) revert ZeroKey();
        if (expiresAt <= block.timestamp) revert InvalidIntent();
        if (limitUsdc == 0 || limitUsdc > type(uint128).max) revert InvalidLimit();

        Layout storage $ = _layout();
        _indexStrategy($, strategyId);
        _indexSession($, strategyId, key);

        Session storage session = $.sessions[strategyId][key];
        if (session.exists) revert SessionAlreadyExists();

        session.limitUsdc = uint128(limitUsdc);
        session.capacityUsdc = uint128(limitUsdc);
        session.deployedUsdc = 0;
        session.expiresAt = uint64(expiresAt);
        session.nonce = 0;
        session.revoked = false;
        session.exists = true;

        emit SessionGranted(strategyId, key, limitUsdc, expiresAt);
    }

    function executeSwap(
        SwapIntent calldata intent,
        bytes calldata routerCalldata,
        bytes calldata sessionSignature
    ) external nonReentrant {
        bytes memory routerData = routerCalldata;
        _validateRouterCalldata(
            routerData, intent.routerCalldataHash, intent.sellToken, intent.maxSellAmount
        );
        _validateSignedSwap(
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            _swapIntentDigest(intent),
            sessionSignature
        );

        (uint256 sellAmount, uint256 buyAmount) =
            _executeRouterSwap(intent.sellToken, intent.maxSellAmount, routerData);
        if (buyAmount < intent.minBuyAmount) revert SlippageExceeded();

        _completeSwap(
            _layout(),
            intent.strategyId,
            intent.sessionKey,
            intent.sellToken,
            intent.buyToken,
            sellAmount,
            buyAmount
        );
    }

    function executeSwapWithFees(
        SwapBundleIntent calldata intent,
        bytes calldata strategyRouterCalldata,
        bytes calldata gasRouterCalldata,
        bytes calldata sessionSignature
    ) external nonReentrant {
        bytes memory strategyCalldata = strategyRouterCalldata;
        bytes memory gasCalldata = gasRouterCalldata;

        _validateRouterCalldata(
            strategyCalldata, intent.routerCalldataHash, intent.sellToken, intent.maxSellAmount
        );
        _validateGasRouterCalldata(gasCalldata, intent.gasRouterCalldataHash, intent.gasSellUsdc);
        _validateSignedSwap(
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            SwapBundleIntentHash.digest(_bundlePayload(intent), _domainSeparator()),
            sessionSignature
        );

        if (intent.sellToken == usdcToken && intent.buyToken != usdcToken) {
            _executeBuyBundle(_layout(), intent, strategyCalldata, gasCalldata);
            return;
        }
        if (intent.buyToken == usdcToken && intent.sellToken != usdcToken) {
            _executeSellBundle(_layout(), intent, strategyCalldata, gasCalldata);
            return;
        }
        revert InvalidIntent();
    }

    function _executeBuyBundle(
        Layout storage $,
        SwapBundleIntent calldata intent,
        bytes memory strategyRouterCalldata,
        bytes memory gasRouterCalldata
    ) private {
        Session storage session = $.sessions[intent.strategyId][intent.sessionKey];
        uint256 feeCapacityCost =
            _normalizeUsdc(intent.platformFeeUsdc) + _normalizeUsdc(intent.gasSellUsdc);
        uint256 strategyCost = _normalizeUsdc(intent.maxSellAmount);
        uint256 deployable = uint256(session.capacityUsdc) - uint256(session.deployedUsdc);
        if (strategyCost + feeCapacityCost > deployable) revert SpendLimitExceeded();

        if (feeCapacityCost > 0) {
            session.capacityUsdc = uint128(uint256(session.capacityUsdc) - feeCapacityCost);
        }
        _chargePlatformFee(intent);
        _reimburseGas(intent, gasRouterCalldata);

        (uint256 sellAmount, uint256 buyAmount) =
            _executeRouterSwap(intent.sellToken, intent.maxSellAmount, strategyRouterCalldata);
        if (buyAmount < intent.minBuyAmount) revert SlippageExceeded();

        _completeSwap(
            $,
            intent.strategyId,
            intent.sessionKey,
            intent.sellToken,
            intent.buyToken,
            sellAmount,
            buyAmount
        );
    }

    function _executeSellBundle(
        Layout storage $,
        SwapBundleIntent calldata intent,
        bytes memory strategyRouterCalldata,
        bytes memory gasRouterCalldata
    ) private {
        Session storage session = $.sessions[intent.strategyId][intent.sessionKey];
        (uint256 sellAmount, uint256 buyAmount) =
            _executeRouterSwap(intent.sellToken, intent.maxSellAmount, strategyRouterCalldata);
        if (buyAmount < intent.minBuyAmount) revert SlippageExceeded();

        int256 realizedPnlUsdc = _applySwapAccounting(
            $, intent.strategyId, session, intent.sellToken, intent.buyToken, sellAmount, buyAmount
        );

        uint256 feeCapacityCost =
            _normalizeUsdc(intent.platformFeeUsdc) + _normalizeUsdc(intent.gasSellUsdc);
        if (feeCapacityCost > 0) {
            if (feeCapacityCost > session.capacityUsdc) revert SpendLimitExceeded();
            session.capacityUsdc = uint128(uint256(session.capacityUsdc) - feeCapacityCost);
        }
        _chargePlatformFee(intent);
        _reimburseGas(intent, gasRouterCalldata);

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

    function _validateSignedSwap(
        bytes32 strategyId,
        address sessionKey,
        uint256 nonce,
        uint256 deadline,
        bytes32 digest,
        bytes calldata sessionSignature
    ) private view {
        if (block.timestamp > deadline) revert IntentExpired();
        Session storage session = _layout().sessions[strategyId][sessionKey];
        _assertActive(session);
        if (session.nonce != nonce) revert NonceMismatch();
        if (_recover(digest, sessionSignature) != sessionKey) revert InvalidSignature();
    }

    function _completeSwap(
        Layout storage $,
        bytes32 strategyId,
        address sessionKey,
        address sellToken,
        address buyToken,
        uint256 sellAmount,
        uint256 buyAmount
    ) private {
        Session storage session = $.sessions[strategyId][sessionKey];
        int256 realizedPnlUsdc = _applySwapAccounting(
            $, strategyId, session, sellToken, buyToken, sellAmount, buyAmount
        );
        session.nonce += 1;
        emit SwapExecuted(
            strategyId, sessionKey, sellToken, buyToken, sellAmount, buyAmount, realizedPnlUsdc
        );
    }

    function revoke(bytes32 strategyId, address key) external {
        if (msg.sender != address(this) && msg.sender != key) revert NotOwner();
        Layout storage $ = _layout();
        Session storage session = $.sessions[strategyId][key];
        if (!session.exists) revert SessionUnknown();
        _revoke(strategyId, key, session);
    }

    function revokeWithSignature(RevokeIntent calldata intent, bytes calldata sessionSignature)
        external
    {
        if (block.timestamp > intent.deadline) revert IntentExpired();
        Layout storage $ = _layout();
        Session storage session = $.sessions[intent.strategyId][intent.sessionKey];
        _assertActive(session);
        if (session.nonce != intent.nonce) revert NonceMismatch();
        bytes32 digest = _revokeIntentDigest(intent);
        if (_recover(digest, sessionSignature) != intent.sessionKey) revert InvalidSignature();
        _revoke(intent.strategyId, intent.sessionKey, session);
    }

    function _revoke(bytes32 strategyId, address key, Session storage session) private {
        session.revoked = true;
        session.nonce += 1;
        emit SessionRevocation(strategyId, key, session.nonce);
    }

    function rotateSession(bytes32 strategyId, address oldKey, address newKey) external onlyOwner {
        if (oldKey == address(0) || newKey == address(0)) revert ZeroKey();
        Layout storage $ = _layout();
        Session storage oldSession = $.sessions[strategyId][oldKey];
        if (!oldSession.exists) revert SessionUnknown();
        if ($.sessions[strategyId][newKey].exists) revert SessionKeyTaken();

        Session storage newSession = $.sessions[strategyId][newKey];
        newSession.limitUsdc = oldSession.limitUsdc;
        newSession.capacityUsdc = oldSession.capacityUsdc;
        newSession.deployedUsdc = oldSession.deployedUsdc;
        newSession.expiresAt = oldSession.expiresAt;
        newSession.nonce = oldSession.nonce;
        newSession.revoked = false;
        newSession.exists = true;

        oldSession.revoked = true;
        oldSession.exists = false;
        oldSession.nonce += 1;

        _indexSession($, strategyId, newKey);
        emit SessionRotated(strategyId, oldKey, newKey);
    }

    function setLimit(bytes32 strategyId, address key, uint256 newLimitUsdc, uint256 expiresAt)
        external
        onlyOwner
    {
        if (newLimitUsdc == 0 || newLimitUsdc > type(uint128).max) revert InvalidLimit();
        if (expiresAt <= block.timestamp) revert InvalidIntent();
        Layout storage $ = _layout();
        Session storage session = $.sessions[strategyId][key];
        if (!session.exists) revert SessionUnknown();

        uint256 oldLimit = session.limitUsdc;
        if (newLimitUsdc > oldLimit) {
            session.capacityUsdc =
                uint128(uint256(session.capacityUsdc) + (newLimitUsdc - oldLimit));
        } else if (newLimitUsdc < oldLimit) {
            uint256 nextCapacity = uint256(session.capacityUsdc);
            if (nextCapacity > newLimitUsdc) {
                nextCapacity = newLimitUsdc;
            }
            session.capacityUsdc = uint128(nextCapacity);
        }
        session.limitUsdc = uint128(newLimitUsdc);
        session.expiresAt = uint64(expiresAt);
        emit SessionLimitUpdated(strategyId, key, oldLimit, newLimitUsdc, expiresAt);
    }

    function strategyCount() external view returns (uint256) {
        return _layout().strategyIds.length;
    }

    function strategyAt(uint256 index) external view returns (bytes32) {
        return _layout().strategyIds[index];
    }

    function sessionCount(bytes32 strategyId) external view returns (uint256) {
        return _layout().sessionKeys[strategyId].length;
    }

    function sessionAt(bytes32 strategyId, uint256 index) external view returns (address) {
        return _layout().sessionKeys[strategyId][index];
    }

    function sessionOf(bytes32 strategyId, address key) external view returns (Session memory) {
        return _layout().sessions[strategyId][key];
    }

    function assetOf(bytes32 strategyId, address token) external view returns (AssetRecord memory) {
        return _layout().assets[strategyId][token];
    }

    function _applySwapAccounting(
        Layout storage $,
        bytes32 strategyId,
        Session storage session,
        address sellToken,
        address buyToken,
        uint256 sellAmount,
        uint256 buyAmount
    ) private returns (int256 realizedPnlUsdc) {
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
    ) private {
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
    ) private returns (int256 realizedPnlUsdc) {
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

    function _normalizeUsdc(uint256 amount) private view returns (uint256) {
        uint8 decimals = usdcDecimals;
        if (decimals == 6) {
            return amount;
        }
        if (decimals > 6) {
            return amount / (10 ** (decimals - 6));
        }
        return amount * (10 ** (6 - decimals));
    }

    function _routerSelector(bytes memory routerCalldata) private pure returns (bytes4 selector) {
        assembly {
            selector := mload(add(routerCalldata, 32))
        }
    }

    function _validateRouterCalldata(
        bytes memory routerCalldata,
        bytes32 routerCalldataHash,
        address sellToken,
        uint256 maxSellAmount
    ) private pure {
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
            calldataSellToken != sellToken || calldataSellAmount != maxSellAmount
                || operator == address(0) || target == address(0) || operator != target
        ) revert RouterFieldsMismatch();
        if (targetCalldata.length < 4) revert RouterFieldsMismatch();
    }

    function _validateGasRouterCalldata(
        bytes memory gasRouterCalldata,
        bytes32 gasRouterCalldataHash,
        uint256 gasSellUsdc
    ) private view {
        if (gasSellUsdc == 0) {
            if (gasRouterCalldata.length != 0 || gasRouterCalldataHash != bytes32(0)) {
                revert InvalidIntent();
            }
            return;
        }
        _validateRouterCalldata(gasRouterCalldata, gasRouterCalldataHash, usdcToken, gasSellUsdc);
    }

    function _executeRouterSwap(
        address sellToken,
        uint256 maxSellAmount,
        bytes memory routerCalldata
    ) private returns (uint256 sellAmount, uint256 buyAmount) {
        address buyToken = _routerBuyToken(routerCalldata);
        uint256 sellBefore = IERC20(sellToken).balanceOf(address(this));
        uint256 buyBefore = buyToken == address(0)
            ? address(this).balance
            : IERC20(buyToken).balanceOf(address(this));

        _forceApprove(sellToken, ALLOWANCE_HOLDER, maxSellAmount);
        (bool ok, bytes memory result) = ALLOWANCE_HOLDER.call(routerCalldata);
        if (!ok) revert CallFailed(result);
        _forceApprove(sellToken, ALLOWANCE_HOLDER, 0);

        uint256 sellAfter = IERC20(sellToken).balanceOf(address(this));
        uint256 buyAfter = buyToken == address(0)
            ? address(this).balance
            : IERC20(buyToken).balanceOf(address(this));

        sellAmount = sellBefore - sellAfter;
        buyAmount = buyAfter - buyBefore;
        if (sellAmount == 0 || buyAmount == 0) revert InvalidIntent();
        if (sellAmount > maxSellAmount) revert SlippageExceeded();
    }

    function _routerArgs(bytes memory routerCalldata) private pure returns (bytes memory) {
        bytes memory args = new bytes(routerCalldata.length - 4);
        for (uint256 i = 0; i < args.length; ++i) {
            args[i] = routerCalldata[i + 4];
        }
        return args;
    }

    function _routerBuyToken(bytes memory routerCalldata) private pure returns (address buyToken) {
        (,,,, bytes memory targetCalldata) =
            abi.decode(_routerArgs(routerCalldata), (address, address, uint256, address, bytes));
        buyToken = abi.decode(targetCalldata, (address));
    }

    function _chargePlatformFee(SwapBundleIntent calldata intent) private {
        if (intent.platformFeeUsdc == 0) {
            if (intent.feeRecipient != address(0)) revert InvalidIntent();
            return;
        }
        if (intent.feeRecipient == address(0)) revert InvalidIntent();
        if (!IERC20(usdcToken).transfer(intent.feeRecipient, intent.platformFeeUsdc)) {
            revert CallFailed("");
        }
        emit PlatformFeeCharged(
            intent.strategyId, intent.sessionKey, intent.feeRecipient, intent.platformFeeUsdc
        );
    }

    function _reimburseGas(SwapBundleIntent calldata intent, bytes memory gasRouterCalldata)
        private
    {
        if (intent.gasSellUsdc == 0) {
            if (intent.minNativeOut > 0 || intent.gasRecipient != address(0)) {
                revert InvalidIntent();
            }
            return;
        }
        if (intent.gasRecipient == address(0)) revert InvalidIntent();

        uint256 nativeBefore = address(this).balance;
        _executeRouterSwap(usdcToken, intent.gasSellUsdc, gasRouterCalldata);
        uint256 nativeOut = address(this).balance - nativeBefore;
        if (nativeOut < intent.minNativeOut) revert SlippageExceeded();

        (bool sent,) = intent.gasRecipient.call{value: nativeOut}("");
        if (!sent) revert CallFailed("");

        emit GasReimbursed(
            intent.strategyId, intent.sessionKey, intent.gasRecipient, intent.gasSellUsdc, nativeOut
        );
    }

    function _forceApprove(address token, address spender, uint256 amount) private {
        if (_tryApprove(token, spender, amount)) return;
        if (!_tryApprove(token, spender, 0) || !_tryApprove(token, spender, amount)) {
            revert ApprovalFailed(token, amount);
        }
    }

    function _tryApprove(address token, address spender, uint256 amount) private returns (bool) {
        (bool ok, bytes memory result) =
            token.call(abi.encodeCall(IERC20.approve, (spender, amount)));
        if (!ok) return false;
        if (result.length == 0) return true;
        if (result.length < 32) return false;
        uint256 returned;
        assembly {
            returned := mload(add(result, 32))
        }
        return returned == 1;
    }

    function _assertActive(Session storage session) private view {
        if (!session.exists) revert SessionUnknown();
        if (session.revoked) revert SessionRevoked();
        if (session.expiresAt <= block.timestamp) revert SessionExpired();
    }

    function _indexStrategy(Layout storage $, bytes32 strategyId) private {
        if ($.strategyIndex[strategyId] != 0) return;
        $.strategyIds.push(strategyId);
        $.strategyIndex[strategyId] = $.strategyIds.length;
    }

    function _indexSession(Layout storage $, bytes32 strategyId, address key) private {
        if ($.sessionKeyIndex[strategyId][key] != 0) return;
        $.sessionKeys[strategyId].push(key);
        $.sessionKeyIndex[strategyId][key] = $.sessionKeys[strategyId].length;
    }

    function _bundlePayload(SwapBundleIntent calldata intent)
        private
        pure
        returns (SwapBundleIntentPayload memory payload)
    {
        payload = SwapBundleIntentPayload({
            strategyId: intent.strategyId,
            sessionKey: intent.sessionKey,
            nonce: intent.nonce,
            deadline: intent.deadline,
            sellToken: intent.sellToken,
            buyToken: intent.buyToken,
            maxSellAmount: intent.maxSellAmount,
            minBuyAmount: intent.minBuyAmount,
            routerCalldataHash: intent.routerCalldataHash,
            platformFeeUsdc: intent.platformFeeUsdc,
            feeRecipient: intent.feeRecipient,
            gasSellUsdc: intent.gasSellUsdc,
            minNativeOut: intent.minNativeOut,
            gasRecipient: intent.gasRecipient,
            gasRouterCalldataHash: intent.gasRouterCalldataHash
        });
    }

    function _swapIntentDigest(SwapIntent calldata intent) private view returns (bytes32) {
        return keccak256(
            abi.encodePacked(
                "\x19\x01",
                _domainSeparator(),
                keccak256(
                    abi.encode(
                        SWAP_INTENT_TYPEHASH,
                        intent.strategyId,
                        intent.sessionKey,
                        intent.nonce,
                        intent.deadline,
                        intent.sellToken,
                        intent.buyToken,
                        intent.maxSellAmount,
                        intent.minBuyAmount,
                        intent.routerCalldataHash
                    )
                )
            )
        );
    }

    function _revokeIntentDigest(RevokeIntent calldata intent) private view returns (bytes32) {
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

    function _domainSeparator() private view returns (bytes32) {
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

    function _recover(bytes32 digest, bytes calldata signature) private pure returns (address) {
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

    function _layout() private pure returns (Layout storage $) {
        bytes32 slot = STORAGE_LOCATION;
        assembly {
            $.slot := slot
        }
    }
}
