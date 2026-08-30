// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {SessionSpendBase} from "src/SessionSpendBase.sol";

/// @title SessionSpendSwap
/// @notice Delegatecall module for same-chain 0x session swaps.
contract SessionSpendSwap is SessionSpendBase {
    constructor(address usdcToken_) SessionSpendBase(usdcToken_) {}

    function executeSwapWithFeesV2(
        SwapBundleIntentV2 calldata intent,
        bytes calldata strategyRouterCalldata,
        bytes calldata gasRouterCalldata,
        bytes calldata sessionSignature
    ) external nonReentrant {
        bytes memory strategyCalldata = strategyRouterCalldata;
        bytes memory gasCalldata = gasRouterCalldata;
        _validateV2Swap(intent, strategyCalldata, gasCalldata, sessionSignature);

        if (intent.gasFundingMode == GasFundingMode.NATIVE_OUTPUT) {
            _executeNativeOutputBundle(_layout(), intent, strategyCalldata);
            return;
        }
        _executeV2Bundle(_layout(), intent, strategyCalldata, gasCalldata);
    }
}
