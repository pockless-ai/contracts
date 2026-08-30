// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {SessionSpendBase} from "src/SessionSpendBase.sol";
import {SessionSpendRelay} from "src/SessionSpendRelay.sol";
import {SessionSpendSwap} from "src/SessionSpendSwap.sol";
import {StrategyVault} from "src/StrategyVault.sol";

/// @title SessionSpend7702
/// @notice ERC-7702 implementation. Heavy Relay/swap logic lives in constructor-deployed
///         modules and runs via delegatecall so this runtime stays under EIP-170.
contract SessionSpend7702 is SessionSpendBase {
    address private immutable relayModule;
    address private immutable swapModule;

    constructor(address usdcToken_) SessionSpendBase(usdcToken_) {
        relayModule = address(new SessionSpendRelay(usdcToken_));
        swapModule = address(new SessionSpendSwap(usdcToken_));
    }

    function _delegate(address module) private {
        assembly {
            calldatacopy(0, 0, calldatasize())
            let ok := delegatecall(gas(), module, 0, calldatasize(), 0, 0)
            returndatacopy(0, 0, returndatasize())
            if iszero(ok) { revert(0, returndatasize()) }
            return(0, returndatasize())
        }
    }

    function strategyVaultInitCodeHash(address owner_) public pure returns (bytes32) {
        return keccak256(abi.encodePacked(type(StrategyVault).creationCode, abi.encode(owner_)));
    }

    function predictStrategyVault(bytes32 strategyId) public view returns (address) {
        bytes32 salt = keccak256(abi.encode(strategyId));
        bytes32 initCodeHash = strategyVaultInitCodeHash(address(this));
        return address(
            uint160(
                uint256(
                    keccak256(abi.encodePacked(bytes1(0xff), address(this), salt, initCodeHash))
                )
            )
        );
    }

    function setPlatformRelayer(address relayer) external onlyOwner {
        Layout storage $ = _layout();
        address oldRelayer = $.platformRelayer;
        $.platformRelayer = relayer;
        emit PlatformRelayerUpdated(oldRelayer, relayer);
    }

    function platformRelayer() external view returns (address) {
        return _layout().platformRelayer;
    }

    function strategyVaultOf(bytes32 strategyId) external view returns (address) {
        return _layout().strategyVault[strategyId];
    }

    function remoteAssetOf(bytes32 strategyId, address token, uint256 fundingChainId)
        external
        view
        returns (RemoteAssetRecord memory)
    {
        return _layout().remoteAssets[strategyId][token][fundingChainId];
    }

    function vaultAccountedBalanceOf(bytes32 strategyId, address token)
        external
        view
        returns (uint256)
    {
        return _layout().vaultAccountedBalance[strategyId][token];
    }

    function pendingDepositOf(bytes32 relayOrderId) external view returns (PendingDeposit memory) {
        return _layout().pendingDeposits[relayOrderId];
    }

    function pendingSellOf(bytes32 relayOrderId) external view returns (PendingSell memory) {
        return _layout().pendingSells[relayOrderId];
    }

    function pendingGasTopUpOf(bytes32 relayOrderId)
        external
        view
        returns (PendingGasTopUp memory)
    {
        return _layout().pendingGasTopUps[relayOrderId];
    }

    function relayReceiptConsumed(bytes32 strategyId, bytes32 relayOrderId, RelayAction action)
        external
        view
        returns (bool)
    {
        return _layout().consumedRelayReceipts[strategyId][relayOrderId][uint8(action)];
    }

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

        _ensureStrategyVault($, strategyId);

        emit SessionGranted(strategyId, key, limitUsdc, expiresAt);
    }

    function recoverVaultSurplus(
        bytes32 strategyId,
        address token,
        address recipient,
        uint256 amount
    ) external onlyOwner nonReentrant {
        if (recipient == address(0) || amount == 0) revert InvalidIntent();
        Layout storage $ = _layout();
        address vault = _requireStrategyVault($, strategyId);
        uint256 recoverable = _vaultSurplus($, strategyId, token);
        if (amount > recoverable) revert InsufficientVaultSurplus();

        if (token == address(0)) {
            _asVault(vault).transferNative(payable(recipient), amount);
        } else {
            _asVault(vault).transferToken(token, recipient, amount);
        }
        emit VaultSurplusRecovered(strategyId, token, recipient, amount);
    }

    function walletRelayNonce() external view returns (uint256) {
        return _layout().walletRelayNonce;
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

    function executeRelayDeposit(
        RelayDepositIntent calldata intent,
        address relayTarget,
        bytes calldata relayCalldata,
        uint256 relayValue,
        bytes calldata sessionSignature
    ) external {
        _delegate(relayModule);
    }

    function creditRelayAsset(
        CreditRelayAssetIntent calldata intent,
        bytes calldata sessionSignature
    ) external {
        _delegate(relayModule);
    }

    function executeRemoteRelaySell(
        RemoteRelaySellIntent calldata intent,
        address relayTarget,
        bytes calldata relayCalldata,
        uint256 relayValue,
        bytes calldata sessionSignature
    ) external {
        _delegate(relayModule);
    }

    function creditUsdcReturn(
        CreditUsdcReturnIntent calldata intent,
        bytes calldata sessionSignature
    ) external {
        _delegate(relayModule);
    }

    function releaseRelayDeposit(
        ReleaseRelayDepositIntent calldata intent,
        bytes calldata sessionSignature
    ) external {
        _delegate(relayModule);
    }

    function restoreRemoteRelayAsset(
        RestoreRemoteRelayAssetIntent calldata intent,
        bytes calldata sessionSignature
    ) external {
        _delegate(relayModule);
    }

    function executeRelayGasTopUp(
        RelayGasTopUpIntent calldata intent,
        address relayTarget,
        bytes calldata relayCalldata,
        uint256 relayValue,
        bytes calldata sessionSignature
    ) external {
        _delegate(relayModule);
    }

    function releaseRelayGasTopUp(
        ReleaseRelayGasTopUpIntent calldata intent,
        bytes calldata sessionSignature
    ) external {
        _delegate(relayModule);
    }

    function executeWalletRelaySwap(
        WalletRelaySwapIntent calldata intent,
        address relayTarget,
        bytes calldata relayCalldata,
        uint256 relayValue,
        bytes calldata ownerSignature
    ) external payable {
        _delegate(relayModule);
    }

    function executeSwapWithFeesV2(
        SwapBundleIntentV2 calldata intent,
        bytes calldata strategyRouterCalldata,
        bytes calldata gasRouterCalldata,
        bytes calldata sessionSignature
    ) external {
        _delegate(swapModule);
    }
}
