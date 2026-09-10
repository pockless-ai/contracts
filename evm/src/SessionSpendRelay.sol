// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {
    CreditRelayAssetIntentHash,
    RelayDepositIntentHash,
    RelayGasTopUpIntentHash,
    ReleaseRelayDepositIntentHash,
    ReleaseRelayGasTopUpIntentHash,
    RestoreRemoteRelayAssetIntentHash,
    SessionSpendBase
} from "src/SessionSpendBase.sol";

/// @title SessionSpendRelay
/// @notice Delegatecall module for Relay deposit/return/restore and wallet Relay swaps.
contract SessionSpendRelay is SessionSpendBase {
    constructor(address usdcToken_) SessionSpendBase(usdcToken_) {}

    function executeRelayDeposit(
        RelayDepositIntent calldata intent,
        address relayTarget,
        bytes calldata relayCalldata,
        uint256 relayValue,
        bytes calldata sessionSignature
    ) external onlyPlatformRelayer nonReentrant {
        bytes32 relayCallHash = keccak256(abi.encode(relayTarget, relayValue, relayCalldata));
        if (relayCallHash != intent.relayCalldataHash) revert InvalidIntent();
        _validateRelayTarget(relayTarget);
        _validateRelayDecimals(intent.originTokenDecimals, intent.destTokenDecimals);
        if (intent.relayOrderId == bytes32(0)) revert InvalidIntent();

        _validateSignedSwap(
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            RelayDepositIntentHash.digest(_relayDepositPayload(intent), _domainSeparator()),
            sessionSignature
        );
        _consumeRelayReceipt(intent.strategyId, intent.relayOrderId, RelayAction.Deposit);

        Layout storage $ = _layout();
        address vault = _requireStrategyVault($, intent.strategyId);
        _validateRelayDepositRecipients(intent, vault);

        Session storage session = $.sessions[intent.strategyId][intent.sessionKey];
        if (intent.originToken == usdcToken) {
            _lockUsdcRelayDeposit($, session, vault, intent);
        } else {
            _lockRemoteInventoryRelayDeposit($, intent);
        }

        _executeVaultRelayCall(
            vault, intent.originToken, intent.originAmount, relayTarget, relayCalldata, relayValue
        );

        session.nonce += 1;
        _emitRelayDepositExecuted(intent);
    }

    function creditRelayAsset(
        CreditRelayAssetIntent calldata intent,
        bytes calldata sessionSignature
    ) external onlyPlatformRelayer nonReentrant {
        if (intent.relayOrderId == bytes32(0) || intent.creditQuantity == 0) revert InvalidIntent();
        _validateSignedSwap(
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            CreditRelayAssetIntentHash.digest(_creditRelayAssetPayload(intent), _domainSeparator()),
            sessionSignature
        );
        _consumeRelayReceipt(intent.strategyId, intent.relayOrderId, RelayAction.CreditAsset);

        Layout storage $ = _layout();
        _requireStrategyVault($, intent.strategyId);
        uint256 surplus = _vaultSurplus($, intent.strategyId, intent.token);
        if (intent.creditQuantity > surplus) revert InsufficientVaultSurplus();

        RemoteAssetRecord storage remote =
            $.remoteAssets[intent.strategyId][intent.token][intent.fundingChainId];
        remote.quantity += intent.creditQuantity;
        remote.costUsdc = uint128(uint256(remote.costUsdc) + uint256(intent.costUsdc));
        _increaseVaultAccounted($, intent.strategyId, intent.token, intent.creditQuantity);

        Session storage session = $.sessions[intent.strategyId][intent.sessionKey];
        session.nonce += 1;
        emit RelayAssetCredited(
            intent.strategyId,
            intent.sessionKey,
            intent.relayOrderId,
            intent.token,
            intent.fundingChainId,
            intent.creditQuantity,
            intent.costUsdc
        );
    }

    function executeRemoteRelaySell(
        RemoteRelaySellIntent calldata intent,
        address relayTarget,
        bytes calldata relayCalldata,
        uint256 relayValue,
        bytes calldata sessionSignature
    ) external onlyPlatformRelayer nonReentrant {
        address vault = _validateRemoteRelaySell(
            intent, relayTarget, relayCalldata, relayValue, sessionSignature
        );
        uint128 provisionalCost = _stageRemoteRelaySell(intent);
        _executeVaultRelayCall(
            vault, intent.token, intent.sellQuantity, relayTarget, relayCalldata, relayValue
        );
        _finalizeRemoteRelaySell(intent, provisionalCost);
    }

    function creditUsdcReturn(
        CreditUsdcReturnIntent calldata intent,
        bytes calldata sessionSignature
    ) external onlyPlatformRelayer nonReentrant {
        Layout storage $ = _layout();
        PendingSell storage sell = $.pendingSells[intent.relayOrderId];
        // Cross-chain EVM sells write pending on the origin chain. Dest USDC and
        // deployed USDC live on the funding chain, so credit opens the matching
        // pending here from the signed intent when this is the funding chain.
        if (!sell.exists) {
            if (intent.fundingChainId != block.chainid) revert PendingRecordMissing();
            sell.strategyId = intent.strategyId;
            sell.fundingChainId = intent.fundingChainId;
            sell.quantity = intent.destQuantityReleased;
            sell.provisionalCostUsdc = intent.destCostReleasedUsdc;
            sell.exists = true;
        }
        if (sell.strategyId != intent.strategyId) revert PendingRecordMissing();
        if (sell.fundingChainId != intent.fundingChainId) revert InvalidIntent();
        if (intent.destQuantityReleased != sell.quantity) revert InvalidIntent();
        if (uint256(intent.destCostReleasedUsdc) != uint256(sell.provisionalCostUsdc)) {
            revert InvalidIntent();
        }
        address vault = _validateCreditUsdcReturn(intent, sessionSignature);
        int256 realizedPnlUsdc = _applyCreditUsdcReturnAccounting(intent, vault);
        delete $.pendingSells[intent.relayOrderId];
        _finalizeCreditUsdcReturn(intent, realizedPnlUsdc);
    }

    function releaseRelayDeposit(
        ReleaseRelayDepositIntent calldata intent,
        bytes calldata sessionSignature
    ) external onlyPlatformRelayer nonReentrant {
        if (intent.relayOrderId == bytes32(0)) {
            revert InvalidIntent();
        }
        _validateSignedSwap(
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            ReleaseRelayDepositIntentHash.digest(
                _releaseRelayDepositPayload(intent), _domainSeparator()
            ),
            sessionSignature
        );
        _consumeRelayReceipt(intent.strategyId, intent.relayOrderId, RelayAction.DepositRelease);

        Layout storage $ = _layout();
        PendingDeposit storage deposit = $.pendingDeposits[intent.relayOrderId];
        if (!deposit.exists || deposit.strategyId != intent.strategyId) {
            revert PendingRecordMissing();
        }
        if (deposit.originToken != intent.token || deposit.fundingChainId != intent.fundingChainId)
        {
            revert InvalidIntent();
        }
        if (intent.refundQuantity != deposit.originAmount) revert InvalidIntent();
        if (uint256(intent.refundCostUsdc) != uint256(deposit.lockedCostUsdc)) {
            revert InvalidIntent();
        }

        _requireStrategyVault($, intent.strategyId);
        uint256 surplus = _vaultSurplus($, intent.strategyId, intent.token);
        if (intent.refundQuantity > surplus) revert InsufficientVaultSurplus();

        Session storage session = $.sessions[intent.strategyId][intent.sessionKey];
        if (intent.token == usdcToken) {
            uint256 costReleased = _normalizeUsdc(uint256(intent.refundCostUsdc));
            if (costReleased == 0 || costReleased > session.deployedUsdc) revert InvalidIntent();
            session.deployedUsdc = uint128(uint256(session.deployedUsdc) - costReleased);
        }

        delete $.pendingDeposits[intent.relayOrderId];
        session.nonce += 1;
        emit RelayDepositReleased(
            intent.strategyId,
            intent.sessionKey,
            intent.relayOrderId,
            intent.refundQuantity,
            intent.refundCostUsdc
        );
    }

    function restoreRemoteRelayAsset(
        RestoreRemoteRelayAssetIntent calldata intent,
        bytes calldata sessionSignature
    ) external onlyPlatformRelayer nonReentrant {
        if (intent.relayOrderId == bytes32(0)) {
            revert InvalidIntent();
        }
        _validateSignedSwap(
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            RestoreRemoteRelayAssetIntentHash.digest(
                _restoreRemoteRelayAssetPayload(intent), _domainSeparator()
            ),
            sessionSignature
        );
        _consumeRelayReceipt(intent.strategyId, intent.relayOrderId, RelayAction.AssetRestore);

        Layout storage $ = _layout();
        PendingSell storage sell = $.pendingSells[intent.relayOrderId];
        if (!sell.exists || sell.strategyId != intent.strategyId) revert PendingRecordMissing();
        if (sell.token != intent.token || sell.fundingChainId != intent.fundingChainId) {
            revert InvalidIntent();
        }
        if (intent.restoreQuantity != sell.quantity) revert InvalidIntent();
        if (uint256(intent.restoreCostUsdc) != uint256(sell.provisionalCostUsdc)) {
            revert InvalidIntent();
        }

        _requireStrategyVault($, intent.strategyId);
        uint256 surplus = _vaultSurplus($, intent.strategyId, intent.token);
        if (intent.restoreQuantity > surplus) revert InsufficientVaultSurplus();

        RemoteAssetRecord storage remote =
            $.remoteAssets[intent.strategyId][intent.token][intent.fundingChainId];
        remote.quantity += intent.restoreQuantity;
        remote.costUsdc = uint128(uint256(remote.costUsdc) + uint256(intent.restoreCostUsdc));
        _increaseVaultAccounted($, intent.strategyId, intent.token, intent.restoreQuantity);

        delete $.pendingSells[intent.relayOrderId];
        Session storage session = $.sessions[intent.strategyId][intent.sessionKey];
        session.nonce += 1;
        emit RemoteRelayAssetRestored(
            intent.strategyId,
            intent.sessionKey,
            intent.relayOrderId,
            intent.token,
            intent.restoreQuantity,
            intent.restoreCostUsdc
        );
    }

    function executeRelayGasTopUp(
        RelayGasTopUpIntent calldata intent,
        address relayTarget,
        bytes calldata relayCalldata,
        uint256 relayValue,
        bytes calldata sessionSignature
    ) external onlyPlatformRelayer nonReentrant {
        bytes32 relayCallHash = keccak256(abi.encode(relayTarget, relayValue, relayCalldata));
        if (relayCallHash != intent.relayCalldataHash) revert InvalidIntent();
        _validateRelayTarget(relayTarget);
        if (intent.relayOrderId == bytes32(0) || intent.overheadUsdc == 0) revert InvalidIntent();
        if (intent.gasRecipient == address(0)) revert InvalidIntent();

        _validateSignedSwap(
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            RelayGasTopUpIntentHash.digest(_relayGasTopUpPayload(intent), _domainSeparator()),
            sessionSignature
        );
        _consumeRelayReceipt(intent.strategyId, intent.relayOrderId, RelayAction.GasTopUp);

        Layout storage $ = _layout();
        address vault = _requireStrategyVault($, intent.strategyId);
        Session storage session = $.sessions[intent.strategyId][intent.sessionKey];
        uint256 overheadUsdc = _normalizeUsdc(intent.overheadUsdc);
        _deductOverhead(session, overheadUsdc);
        _transferToVault(vault, usdcToken, intent.overheadUsdc);
        $.pendingGasTopUps[intent.relayOrderId] = PendingGasTopUp({
            strategyId: intent.strategyId,
            overheadUsdc: uint128(overheadUsdc),
            gasRecipient: intent.gasRecipient,
            exists: true
        });

        _executeVaultRelayCall(
            vault, usdcToken, intent.overheadUsdc, relayTarget, relayCalldata, relayValue
        );

        session.nonce += 1;
        emit RelayGasTopUpExecuted(
            intent.strategyId,
            intent.sessionKey,
            intent.relayOrderId,
            intent.overheadUsdc,
            intent.gasRecipient
        );
    }

    function releaseRelayGasTopUp(
        ReleaseRelayGasTopUpIntent calldata intent,
        bytes calldata sessionSignature
    ) external onlyPlatformRelayer nonReentrant {
        if (intent.relayOrderId == bytes32(0)) {
            revert InvalidIntent();
        }
        _validateSignedSwap(
            intent.strategyId,
            intent.sessionKey,
            intent.nonce,
            intent.deadline,
            ReleaseRelayGasTopUpIntentHash.digest(
                _releaseRelayGasTopUpPayload(intent), _domainSeparator()
            ),
            sessionSignature
        );
        _consumeRelayReceipt(intent.strategyId, intent.relayOrderId, RelayAction.GasTopUpRelease);

        Layout storage $ = _layout();
        PendingGasTopUp storage pending = $.pendingGasTopUps[intent.relayOrderId];
        if (!pending.exists || pending.strategyId != intent.strategyId) {
            revert PendingRecordMissing();
        }
        if (uint256(pending.overheadUsdc) != _normalizeUsdc(intent.refundUsdc)) {
            revert InvalidIntent();
        }

        _requireStrategyVault($, intent.strategyId);
        Session storage session = $.sessions[intent.strategyId][intent.sessionKey];
        session.capacityUsdc =
            uint128(uint256(session.capacityUsdc) + uint256(pending.overheadUsdc));
        delete $.pendingGasTopUps[intent.relayOrderId];
        session.nonce += 1;
        emit RelayGasTopUpReleased(
            intent.strategyId, intent.sessionKey, intent.relayOrderId, intent.refundUsdc
        );
    }

    function executeWalletRelaySwap(
        WalletRelaySwapIntent calldata intent,
        address relayTarget,
        bytes calldata relayCalldata,
        uint256 relayValue,
        bytes calldata ownerSignature
    ) external payable nonReentrant {
        uint256 relayAmount = _validateWalletRelaySwap(
            intent, relayTarget, relayCalldata, relayValue, ownerSignature
        );
        _executeRelayCall(intent.sellToken, relayAmount, relayTarget, relayCalldata, relayValue);
        _finalizeWalletRelaySwap(intent);
    }
}
