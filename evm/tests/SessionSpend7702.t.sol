// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {SessionSpend7702} from "../src/SessionSpend7702.sol";

contract MockERC20 {
    string public name;
    string public symbol;
    uint8 public decimals;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    constructor(string memory name_, string memory symbol_, uint8 decimals_) {
        name = name_;
        symbol = symbol_;
        decimals = decimals_;
    }

    function mint(address to, uint256 amount) external {
        balanceOf[to] += amount;
    }

    function approve(address spender, uint256 amount) public virtual returns (bool) {
        allowance[msg.sender][spender] = amount;
        return true;
    }

    function transfer(address to, uint256 amount) external returns (bool) {
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        uint256 allowed = allowance[from][msg.sender];
        if (allowed != type(uint256).max) {
            allowance[from][msg.sender] = allowed - amount;
        }
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        return true;
    }
}

contract MockUsdt is MockERC20 {
    constructor() MockERC20("Tether USD", "USDT", 6) {}

    function approve(address spender, uint256 amount) public override returns (bool) {
        if (amount != 0 && allowance[msg.sender][spender] != 0) {
            return false;
        }
        allowance[msg.sender][spender] = amount;
        return true;
    }
}

/// @dev Simulates 0x AllowanceHolder.exec at the pinned mainnet address.
contract MockAllowanceHolder {
    mapping(address => mapping(address => uint256)) public rateNumerator;
    mapping(address => mapping(address => uint256)) public observedAllowance;
    bool public pullExtra;
    bool public revertAfterPull;

    function setRate(address sellToken, address buyToken, uint256 numerator) external {
        rateNumerator[sellToken][buyToken] = numerator;
    }

    function setAttack(bool pullExtra_, bool revertAfterPull_) external {
        pullExtra = pullExtra_;
        revertAfterPull = revertAfterPull_;
    }

    function exec(address, address token, uint256 amount, address, bytes calldata data)
        external
        returns (bytes memory)
    {
        (, address buyToken) = abi.decode(data, (bytes4, address));
        uint256 numerator = rateNumerator[token][buyToken];
        require(numerator > 0, "rate");
        observedAllowance[token][msg.sender] = MockERC20(token).allowance(msg.sender, address(this));
        MockERC20(token).transferFrom(msg.sender, address(this), amount + (pullExtra ? 1 : 0));
        if (revertAfterPull) revert("router failed");
        if (buyToken == address(0)) {
            uint256 nativeOut = (amount * numerator) / 1e18;
            (bool sent,) = msg.sender.call{value: nativeOut}("");
            require(sent, "native send failed");
            return "";
        }
        uint256 buyAmount = (amount * numerator) / 1e18;
        MockERC20(buyToken).transfer(msg.sender, buyAmount);
        return "";
    }

    function evil() external pure {}
}

contract SessionSpend7702Test is Test {
    address internal constant ALLOWANCE_HOLDER = 0x0000000000001fF3684f28c67538d4D072C22734;

    SessionSpend7702 internal wallet;
    MockERC20 internal usdc;
    MockERC20 internal weth;
    MockAllowanceHolder internal holder;

    bytes32 internal constant STRATEGY_A = keccak256("strategy-a");
    bytes32 internal constant STRATEGY_B = keccak256("strategy-b");

    uint256 internal sessionKeyPrivateKey = 0xA11CE;
    address internal sessionKey;

    uint256 internal constant LIMIT_USDC = 1_000_000_000; // 1_000 USDC (6 decimals)
    uint256 internal constant EXPIRES_AT = 4_102_444_800;

    address internal feeRecipient = address(0xFEE);
    address internal gasRecipient = address(0x600D);
    uint256 internal constant PLATFORM_FEE = 1_000_000; // 1 USDC
    uint256 internal constant GAS_SELL = 500_000; // 0.5 USDC
    uint256 internal constant NATIVE_RATE = 1e27; // 0.001 ETH per USDC base unit

    function setUp() public {
        sessionKey = vm.addr(sessionKeyPrivateKey);

        usdc = new MockERC20("USD Coin", "USDC", 6);
        weth = new MockERC20("Wrapped Ether", "WETH", 18);
        holder = new MockAllowanceHolder();

        vm.etch(ALLOWANCE_HOLDER, address(holder).code);
        _setRate(address(usdc), address(weth), 1e27);
        _setRate(address(weth), address(usdc), 1e9);
        _setRate(address(usdc), address(0), NATIVE_RATE);

        SessionSpend7702 implementation = new SessionSpend7702(address(usdc));
        address delegatedEoa = vm.addr(0x7702);
        vm.etch(delegatedEoa, address(implementation).code);
        wallet = SessionSpend7702(payable(delegatedEoa));

        usdc.mint(address(wallet), 10_000_000_000);
        usdc.mint(ALLOWANCE_HOLDER, 10_000_000_000);
        weth.mint(ALLOWANCE_HOLDER, 1_000 ether);
        vm.deal(ALLOWANCE_HOLDER, 1_000 ether);

        vm.prank(address(wallet));
        wallet.grant(STRATEGY_A, sessionKey, LIMIT_USDC, EXPIRES_AT);
    }

    // ── grant ────────────────────────────────────────────────────────────────

    function testGrantInitializesSessionState() public view {
        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertTrue(session.exists);
        assertFalse(session.revoked);
        assertEq(session.limitUsdc, LIMIT_USDC);
        assertEq(session.capacityUsdc, LIMIT_USDC);
        assertEq(session.deployedUsdc, 0);
        assertEq(session.expiresAt, EXPIRES_AT);
        assertEq(session.nonce, 0);
    }

    function testGrantEmitsSessionGranted() public {
        address newKey = vm.addr(0xDEAD);
        vm.expectEmit(true, true, false, true);
        emit SessionSpend7702.SessionGranted(STRATEGY_B, newKey, LIMIT_USDC, EXPIRES_AT);
        vm.prank(address(wallet));
        wallet.grant(STRATEGY_B, newKey, LIMIT_USDC, EXPIRES_AT);
    }

    function testGrantRejectsZeroStrategy() public {
        vm.prank(address(wallet));
        vm.expectRevert(SessionSpend7702.ZeroStrategy.selector);
        wallet.grant(bytes32(0), sessionKey, LIMIT_USDC, EXPIRES_AT);
    }

    function testGrantRejectsZeroKey() public {
        vm.prank(address(wallet));
        vm.expectRevert(SessionSpend7702.ZeroKey.selector);
        wallet.grant(STRATEGY_B, address(0), LIMIT_USDC, EXPIRES_AT);
    }

    function testGrantRejectsExpiredTimestamp() public {
        vm.prank(address(wallet));
        vm.expectRevert(SessionSpend7702.InvalidIntent.selector);
        wallet.grant(STRATEGY_B, vm.addr(0x1), LIMIT_USDC, block.timestamp);
    }

    function testGrantRejectsZeroLimit() public {
        vm.prank(address(wallet));
        vm.expectRevert(SessionSpend7702.InvalidLimit.selector);
        wallet.grant(STRATEGY_B, vm.addr(0x1), 0, EXPIRES_AT);
    }

    function testGrantRejectsExistingSession() public {
        vm.prank(address(wallet));
        vm.expectRevert(SessionSpend7702.SessionAlreadyExists.selector);
        wallet.grant(STRATEGY_A, sessionKey, LIMIT_USDC, EXPIRES_AT);
    }

    function testGrantAfterRevokeStillRejected() public {
        vm.prank(sessionKey);
        wallet.revoke(STRATEGY_A, sessionKey);

        vm.prank(address(wallet));
        vm.expectRevert(SessionSpend7702.SessionAlreadyExists.selector);
        wallet.grant(STRATEGY_A, sessionKey, LIMIT_USDC, EXPIRES_AT);
    }

    // ── revoke ───────────────────────────────────────────────────────────────

    function testRevokeBySessionKeyBlocksSwapAndIncrementsNonce() public {
        vm.prank(sessionKey);
        vm.expectEmit(true, true, false, true);
        emit SessionSpend7702.SessionRevocation(STRATEGY_A, sessionKey, 1);
        wallet.revoke(STRATEGY_A, sessionKey);

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertTrue(session.revoked);
        assertEq(session.nonce, 1);

        _expectSwapReverts(SessionSpend7702.SessionRevoked.selector, 0);
    }

    function testRevokeByOwnerBlocksSwap() public {
        vm.prank(address(wallet));
        wallet.revoke(STRATEGY_A, sessionKey);

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertTrue(session.revoked);
        _expectSwapReverts(SessionSpend7702.SessionRevoked.selector, 0);
    }

    function testRevokeRejectsUnknownSession() public {
        vm.prank(address(wallet));
        vm.expectRevert(SessionSpend7702.SessionUnknown.selector);
        wallet.revoke(STRATEGY_A, vm.addr(0x999));
    }

    function testRelayedSessionSignedRevokeBlocksSwap() public {
        SessionSpend7702.RevokeIntent memory intent = SessionSpend7702.RevokeIntent({
            strategyId: STRATEGY_A, sessionKey: sessionKey, nonce: 0, deadline: EXPIRES_AT
        });

        vm.prank(vm.addr(0xB0B));
        wallet.revokeWithSignature(intent, _signRevokeIntent(intent));

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertTrue(session.revoked);
        assertEq(session.nonce, 1);
    }

    function testRelayedRevokeRejectsReplay() public {
        SessionSpend7702.RevokeIntent memory intent = SessionSpend7702.RevokeIntent({
            strategyId: STRATEGY_A, sessionKey: sessionKey, nonce: 0, deadline: EXPIRES_AT
        });
        bytes memory signature = _signRevokeIntent(intent);
        wallet.revokeWithSignature(intent, signature);

        vm.expectRevert(SessionSpend7702.SessionRevoked.selector);
        wallet.revokeWithSignature(intent, signature);
    }

    // ── rotate ───────────────────────────────────────────────────────────────

    function testRotateSessionTransfersState() public {
        _buy(100_000_000, 0.09 ether);

        address newKey = vm.addr(0xC0FFEE);
        vm.prank(address(wallet));
        vm.expectEmit(true, true, true, false);
        emit SessionSpend7702.SessionRotated(STRATEGY_A, sessionKey, newKey);
        wallet.rotateSession(STRATEGY_A, sessionKey, newKey);

        SessionSpend7702.Session memory oldSession = wallet.sessionOf(STRATEGY_A, sessionKey);
        SessionSpend7702.Session memory newSession = wallet.sessionOf(STRATEGY_A, newKey);
        assertFalse(oldSession.exists);
        assertTrue(oldSession.revoked);
        assertTrue(newSession.exists);
        assertFalse(newSession.revoked);
        assertEq(newSession.limitUsdc, LIMIT_USDC);
        assertEq(newSession.capacityUsdc, LIMIT_USDC);
        assertEq(newSession.deployedUsdc, 100_000_000);
        assertEq(newSession.nonce, 1);

        SessionSpend7702.AssetRecord memory wethAsset = wallet.assetOf(STRATEGY_A, address(weth));
        assertEq(wethAsset.quantity, 0.1 ether);
    }

    function testRotateRejectsTakenNewKey() public {
        address takenKey = vm.addr(0xBEEF);
        vm.prank(address(wallet));
        wallet.grant(STRATEGY_A, takenKey, LIMIT_USDC, EXPIRES_AT);

        vm.prank(address(wallet));
        vm.expectRevert(SessionSpend7702.SessionKeyTaken.selector);
        wallet.rotateSession(STRATEGY_A, sessionKey, takenKey);
    }

    function testRotateRejectsUnknownOldKey() public {
        vm.prank(address(wallet));
        vm.expectRevert(SessionSpend7702.SessionUnknown.selector);
        wallet.rotateSession(STRATEGY_A, vm.addr(0x999), vm.addr(0xC0FFEE));
    }

    // ── setLimit ─────────────────────────────────────────────────────────────

    function testSetLimitDecreasesCapacity() public {
        vm.prank(address(wallet));
        vm.expectEmit(true, true, false, true);
        emit SessionSpend7702.SessionLimitUpdated(
            STRATEGY_A, sessionKey, LIMIT_USDC, 500_000_000, EXPIRES_AT
        );
        wallet.setLimit(STRATEGY_A, sessionKey, 500_000_000, EXPIRES_AT);

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.limitUsdc, 500_000_000);
        assertEq(session.capacityUsdc, 500_000_000);
    }

    function testSetLimitIncreasesCapacity() public {
        _buy(200_000_000, 0.18 ether);
        _setRate(address(weth), address(usdc), 9e8);
        _sell(0.2 ether, 1);

        SessionSpend7702.Session memory beforeLimit = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(beforeLimit.capacityUsdc, LIMIT_USDC - 20_000_000);

        vm.prank(address(wallet));
        wallet.setLimit(STRATEGY_A, sessionKey, 1_500_000_000, EXPIRES_AT);

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.limitUsdc, 1_500_000_000);
        assertEq(session.capacityUsdc, 1_480_000_000);
    }

    function testSetLimitClampsCapacityWhenDeployedExceedsNewLimit() public {
        _buy(800_000_000, 0.79 ether);

        vm.prank(address(wallet));
        wallet.setLimit(STRATEGY_A, sessionKey, 500_000_000, EXPIRES_AT);

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.limitUsdc, 500_000_000);
        assertEq(session.capacityUsdc, 500_000_000);
        assertEq(session.deployedUsdc, 800_000_000);
    }

    function testSetLimitRejectsUnknownSession() public {
        vm.prank(address(wallet));
        vm.expectRevert(SessionSpend7702.SessionUnknown.selector);
        wallet.setLimit(STRATEGY_A, vm.addr(0x999), LIMIT_USDC, EXPIRES_AT);
    }

    // ── executeSwap buy accounting ───────────────────────────────────────────

    function testBuyUpdatesDeployedUsdcAndAssetCost() public {
        uint256 spend = 100_000_000;
        _buy(spend, 0.09 ether);

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.deployedUsdc, spend);
        assertEq(session.capacityUsdc, LIMIT_USDC);
        assertEq(usdc.balanceOf(address(wallet)), 10_000_000_000 - spend);

        SessionSpend7702.AssetRecord memory asset = wallet.assetOf(STRATEGY_A, address(weth));
        assertEq(asset.quantity, 0.1 ether);
        assertEq(asset.costUsdc, uint128(spend));
    }

    function testBuyNeedsNoPreExistingAllowanceAndResetsAfterSuccess() public {
        uint256 spend = 100_000_000;
        assertEq(usdc.allowance(address(wallet), ALLOWANCE_HOLDER), 0);

        _buy(spend, 0.09 ether);

        assertEq(_observedAllowance(address(usdc), address(wallet)), spend);
        assertEq(usdc.allowance(address(wallet), ALLOWANCE_HOLDER), 0);
    }

    function testUsdtStyleZeroFirstApproveWorks() public {
        MockUsdt usdt = new MockUsdt();
        SessionSpend7702 implementation = new SessionSpend7702(address(usdt));
        address delegatedEoa = vm.addr(0x7703);
        vm.etch(delegatedEoa, address(implementation).code);

        wallet = SessionSpend7702(payable(delegatedEoa));
        usdc = MockERC20(address(usdt));
        usdt.mint(delegatedEoa, 1_000_000_000);
        _setRate(address(usdt), address(weth), 1e27);

        vm.prank(delegatedEoa);
        wallet.grant(STRATEGY_A, sessionKey, LIMIT_USDC, EXPIRES_AT);
        vm.prank(delegatedEoa);
        usdt.approve(ALLOWANCE_HOLDER, 1);

        _buy(100_000_000, 0.09 ether);

        assertEq(_observedAllowance(address(usdt), delegatedEoa), 100_000_000);
        assertEq(usdt.allowance(delegatedEoa, ALLOWANCE_HOLDER), 0);
    }

    function testBuySpendLimitExceeded() public {
        uint256 spend = LIMIT_USDC + 1;
        _approveForHolder(address(usdc), spend);

        SessionSpend7702.SwapIntent memory intent = _buyIntent(spend, 1, 0);
        bytes memory routerCalldata = _execCalldata(address(usdc), spend, address(weth));
        intent.routerCalldataHash = keccak256(routerCalldata);

        vm.expectRevert(SessionSpend7702.SpendLimitExceeded.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));
    }

    function testBuyCumulativeSpendRespectsRemainingCapacity() public {
        _buy(600_000_000, 0.59 ether);

        uint256 remaining = LIMIT_USDC - 600_000_000;
        _approveForHolder(address(usdc), remaining + 1);

        SessionSpend7702.SwapIntent memory intent = _buyIntent(remaining + 1, 1, 1);
        bytes memory routerCalldata = _execCalldata(address(usdc), remaining + 1, address(weth));
        intent.routerCalldataHash = keccak256(routerCalldata);

        vm.expectRevert(SessionSpend7702.SpendLimitExceeded.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));
    }

    // ── executeSwap sell accounting ──────────────────────────────────────────

    function testSellProfitReplenishesCapacityUpToLimit() public {
        _buy(200_000_000, 0.2 ether);

        _setRate(address(weth), address(usdc), 9e8);
        _sell(0.2 ether, 1);

        SessionSpend7702.Session memory afterLoss = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(afterLoss.deployedUsdc, 0);
        assertEq(afterLoss.capacityUsdc, LIMIT_USDC - 20_000_000);

        _setRate(address(weth), address(usdc), 12e8);
        _buy(180_000_000, 0.18 ether);
        _sell(0.18 ether, 2);

        SessionSpend7702.Session memory afterProfit = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(afterProfit.capacityUsdc, LIMIT_USDC);
    }

    function testSellLossReducesCapacity() public {
        _buy(200_000_000, 0.2 ether);

        _setRate(address(weth), address(usdc), 9e8);
        _sell(0.2 ether, 1);

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.deployedUsdc, 0);
        assertEq(session.capacityUsdc, LIMIT_USDC - 20_000_000);

        SessionSpend7702.AssetRecord memory asset = wallet.assetOf(STRATEGY_A, address(weth));
        assertEq(asset.quantity, 0);
        assertEq(asset.costUsdc, 0);
    }

    function testInventorySellNeedsNoPreExistingAllowanceAndResetsAfterSuccess() public {
        _buy(200_000_000, 0.2 ether);
        assertEq(weth.allowance(address(wallet), ALLOWANCE_HOLDER), 0);

        _sell(0.2 ether, 1);

        assertEq(_observedAllowance(address(weth), address(wallet)), 0.2 ether);
        assertEq(weth.allowance(address(wallet), ALLOWANCE_HOLDER), 0);
    }

    function testSellPartialReleaseProportionalCost() public {
        _buy(200_000_000, 0.2 ether);

        _setRate(address(weth), address(usdc), 1e9);
        _sell(0.1 ether, 90_000_000);

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.deployedUsdc, 100_000_000);

        SessionSpend7702.AssetRecord memory asset = wallet.assetOf(STRATEGY_A, address(weth));
        assertEq(asset.quantity, 0.1 ether);
        assertEq(asset.costUsdc, 100_000_000);
    }

    function testInsufficientInventoryOnSell() public {
        _buy(100_000_000, 0.09 ether);
        weth.mint(address(wallet), 0.1 ether);

        _approveForHolder(address(weth), 0.2 ether);
        SessionSpend7702.SwapIntent memory intent = SessionSpend7702.SwapIntent({
            strategyId: STRATEGY_A,
            sessionKey: sessionKey,
            nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
            deadline: EXPIRES_AT,
            sellToken: address(weth),
            buyToken: address(usdc),
            maxSellAmount: 0.2 ether,
            minBuyAmount: 1,
            routerCalldataHash: bytes32(0)
        });
        bytes memory routerCalldata = _execCalldata(address(weth), 0.2 ether, address(usdc));
        intent.routerCalldataHash = keccak256(routerCalldata);

        vm.expectRevert(SessionSpend7702.InsufficientInventory.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));
    }

    function testStrategyIsolationInventory() public {
        uint256 sessionBPrivateKey = 0xBEEF;
        address sessionB = vm.addr(sessionBPrivateKey);
        vm.prank(address(wallet));
        wallet.grant(STRATEGY_B, sessionB, LIMIT_USDC, EXPIRES_AT);

        _buy(100_000_000, 0.09 ether);
        weth.mint(address(wallet), 0.1 ether);

        _approveForHolder(address(weth), 0.1 ether);
        SessionSpend7702.SwapIntent memory intent = SessionSpend7702.SwapIntent({
            strategyId: STRATEGY_B,
            sessionKey: sessionB,
            nonce: 0,
            deadline: EXPIRES_AT,
            sellToken: address(weth),
            buyToken: address(usdc),
            maxSellAmount: 0.1 ether,
            minBuyAmount: 1,
            routerCalldataHash: bytes32(0)
        });
        bytes memory routerCalldata = _execCalldata(address(weth), 0.1 ether, address(usdc));
        intent.routerCalldataHash = keccak256(routerCalldata);

        vm.expectRevert(SessionSpend7702.InsufficientInventory.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntentWithKey(sessionBPrivateKey, intent));
    }

    // ── nonce replay ─────────────────────────────────────────────────────────

    function testReplayRejectedAfterSuccessfulSwap() public {
        uint256 spend = 50_000_000;
        _approveForHolder(address(usdc), spend);

        SessionSpend7702.SwapIntent memory intent = _buyIntent(spend, 0.045 ether, 0);
        bytes memory routerCalldata = _execCalldata(address(usdc), spend, address(weth));
        intent.routerCalldataHash = keccak256(routerCalldata);
        bytes memory signature = _signIntent(intent);

        wallet.executeSwap(intent, routerCalldata, signature);

        vm.expectRevert(SessionSpend7702.NonceMismatch.selector);
        wallet.executeSwap(intent, routerCalldata, signature);
    }

    function testWrongNonceRejectedBeforeSwap() public {
        _approveForHolder(address(usdc), 50_000_000);

        SessionSpend7702.SwapIntent memory intent = _buyIntent(50_000_000, 0.045 ether, 99);
        bytes memory routerCalldata = _execCalldata(address(usdc), 50_000_000, address(weth));
        intent.routerCalldataHash = keccak256(routerCalldata);

        vm.expectRevert(SessionSpend7702.NonceMismatch.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));
    }

    // ── expired / revoked session ────────────────────────────────────────────

    function testExpiredSessionRejectsSwap() public {
        vm.warp(EXPIRES_AT + 1);
        _approveForHolder(address(usdc), 10_000_000);

        SessionSpend7702.SwapIntent memory intent = _buyIntent(10_000_000, 0.009 ether, 0);
        intent.deadline = EXPIRES_AT + 1000;
        bytes memory routerCalldata = _execCalldata(address(usdc), 10_000_000, address(weth));
        intent.routerCalldataHash = keccak256(routerCalldata);

        vm.expectRevert(SessionSpend7702.SessionExpired.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));
    }

    function testRevokedSessionRejectsSwap() public {
        vm.prank(sessionKey);
        wallet.revoke(STRATEGY_A, sessionKey);
        _expectSwapReverts(SessionSpend7702.SessionRevoked.selector, 1);
    }

    function testIntentExpiredRejectsSwap() public {
        _approveForHolder(address(usdc), 50_000_000);

        SessionSpend7702.SwapIntent memory intent = _buyIntent(50_000_000, 0.045 ether, 0);
        intent.deadline = block.timestamp - 1;
        bytes memory routerCalldata = _execCalldata(address(usdc), 50_000_000, address(weth));
        intent.routerCalldataHash = keccak256(routerCalldata);

        vm.expectRevert(SessionSpend7702.IntentExpired.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));
    }

    // ── router calldata policy ───────────────────────────────────────────────

    function testRouterCalldataHashMismatchRejectsSwap() public {
        _approveForHolder(address(usdc), 100_000_000);

        SessionSpend7702.SwapIntent memory intent = _buyIntent(100_000_000, 0.09 ether, 0);
        bytes memory signedCalldata = _execCalldata(address(usdc), 100_000_000, address(weth));
        intent.routerCalldataHash = keccak256(signedCalldata);

        bytes memory maliciousCalldata = _execCalldata(address(usdc), 200_000_000, address(weth));

        vm.expectRevert(SessionSpend7702.InvalidIntent.selector);
        wallet.executeSwap(intent, maliciousCalldata, _signIntent(intent));
    }

    function testSelectorPolicyRejectsNonExec() public {
        _approveForHolder(address(usdc), 100_000_000);

        SessionSpend7702.SwapIntent memory intent = _buyIntent(100_000_000, 0.09 ether, 0);
        bytes memory routerCalldata = abi.encodeWithSignature("evil()");
        intent.routerCalldataHash = keccak256(routerCalldata);

        vm.expectRevert(SessionSpend7702.SelectorNotAllowed.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));
    }

    function testRouterTokenFieldMustMatchIntent() public {
        SessionSpend7702.SwapIntent memory intent = _buyIntent(100_000_000, 1, 0);
        bytes memory routerCalldata = _execCalldata(address(weth), 100_000_000, address(usdc));
        intent.routerCalldataHash = keccak256(routerCalldata);

        vm.expectRevert(SessionSpend7702.RouterFieldsMismatch.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));
    }

    function testRouterAmountFieldMustMatchIntent() public {
        SessionSpend7702.SwapIntent memory intent = _buyIntent(100_000_000, 1, 0);
        bytes memory routerCalldata = _execCalldata(address(usdc), 99_000_000, address(weth));
        intent.routerCalldataHash = keccak256(routerCalldata);

        vm.expectRevert(SessionSpend7702.RouterFieldsMismatch.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));
    }

    function testRouterTargetAndOperatorMustMatch() public {
        SessionSpend7702.SwapIntent memory intent = _buyIntent(100_000_000, 1, 0);
        bytes memory routerCalldata = abi.encodeWithSelector(
            0x2213bc0b,
            address(0x1111),
            address(usdc),
            100_000_000,
            address(0x2222),
            abi.encode(address(weth))
        );
        intent.routerCalldataHash = keccak256(routerCalldata);

        vm.expectRevert(SessionSpend7702.RouterFieldsMismatch.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));
    }

    function testMaliciousRouterCannotPullMoreThanIntentMaximum() public {
        uint256 spend = 100_000_000;
        SessionSpend7702.SwapIntent memory intent = _buyIntent(spend, 1, 0);
        bytes memory routerCalldata = _execCalldata(address(usdc), spend, address(weth));
        intent.routerCalldataHash = keccak256(routerCalldata);
        _setAttack(true, false);

        vm.expectPartialRevert(SessionSpend7702.CallFailed.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));

        assertEq(usdc.allowance(address(wallet), ALLOWANCE_HOLDER), 0);
        assertEq(usdc.balanceOf(address(wallet)), 10_000_000_000);
        assertEq(wallet.sessionOf(STRATEGY_A, sessionKey).nonce, 0);
    }

    function testRouterRevertLeavesNoAllowanceOrSwapState() public {
        uint256 spend = 100_000_000;
        SessionSpend7702.SwapIntent memory intent = _buyIntent(spend, 1, 0);
        bytes memory routerCalldata = _execCalldata(address(usdc), spend, address(weth));
        intent.routerCalldataHash = keccak256(routerCalldata);
        _setAttack(false, true);

        vm.expectPartialRevert(SessionSpend7702.CallFailed.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));

        assertEq(usdc.allowance(address(wallet), ALLOWANCE_HOLDER), 0);
        assertEq(usdc.balanceOf(address(wallet)), 10_000_000_000);
        assertEq(weth.balanceOf(address(wallet)), 0);
        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.nonce, 0);
        assertEq(session.deployedUsdc, 0);
        SessionSpend7702.AssetRecord memory asset = wallet.assetOf(STRATEGY_A, address(weth));
        assertEq(asset.quantity, 0);
        assertEq(asset.costUsdc, 0);
    }

    function testInvalidSignatureRejected() public {
        _approveForHolder(address(usdc), 50_000_000);

        SessionSpend7702.SwapIntent memory intent = _buyIntent(50_000_000, 0.045 ether, 0);
        bytes memory routerCalldata = _execCalldata(address(usdc), 50_000_000, address(weth));
        intent.routerCalldataHash = keccak256(routerCalldata);

        uint256 wrongKey = 0xBAD;
        vm.expectRevert(SessionSpend7702.InvalidSignature.selector);
        wallet.executeSwap(intent, routerCalldata, _signIntentWithKey(wrongKey, intent));
    }

    // ── indexing ─────────────────────────────────────────────────────────────

    function testEnumerableIndexes() public {
        assertEq(wallet.strategyCount(), 1);
        assertEq(wallet.strategyAt(0), STRATEGY_A);
        assertEq(wallet.sessionCount(STRATEGY_A), 1);
        assertEq(wallet.sessionAt(STRATEGY_A, 0), sessionKey);
    }

    // ── executeSwapWithFees buy ──────────────────────────────────────────────

    function testBuyWithFeesChargesTreasuryAndGasRecipient() public {
        uint256 spend = 100_000_000;
        uint256 feeTotal = PLATFORM_FEE + GAS_SELL;
        uint256 nativeOut = (GAS_SELL * NATIVE_RATE) / 1e18;

        _buyWithFees(spend, 0.09 ether, PLATFORM_FEE, GAS_SELL, nativeOut);

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.deployedUsdc, spend);
        assertEq(session.capacityUsdc, LIMIT_USDC - feeTotal);
        assertEq(usdc.balanceOf(feeRecipient), PLATFORM_FEE);
        assertEq(gasRecipient.balance, nativeOut);
        assertEq(usdc.balanceOf(address(wallet)), 10_000_000_000 - spend - feeTotal);
    }

    function testGasRecipientMustBeTransactionBroadcaster() public {
        uint256 spend = 100_000_000;
        SessionSpend7702.SwapBundleIntent memory intent = _buyBundleIntent(
            spend, 1, wallet.sessionOf(STRATEGY_A, sessionKey).nonce, PLATFORM_FEE, GAS_SELL, 1
        );
        bytes memory strategyCalldata = _execCalldata(address(usdc), spend, address(weth));
        bytes memory gasCalldata = _execCalldata(address(usdc), GAS_SELL, address(0));
        intent.routerCalldataHash = keccak256(strategyCalldata);
        intent.gasRouterCalldataHash = keccak256(gasCalldata);

        vm.prank(address(0xBAD));
        vm.expectRevert(SessionSpend7702.InvalidIntent.selector);
        wallet.executeSwapWithFees(intent, strategyCalldata, gasCalldata, _signBundleIntent(intent));
    }

    function testBuyWithFeesAllInLimitIncludesStrategyFeeAndGas() public {
        uint256 spend = LIMIT_USDC - PLATFORM_FEE - GAS_SELL;
        _approveForHolder(address(usdc), spend);

        SessionSpend7702.SwapBundleIntent memory intent = _buyBundleIntent(
            spend, 1, wallet.sessionOf(STRATEGY_A, sessionKey).nonce, PLATFORM_FEE, GAS_SELL, 1
        );
        bytes memory strategyCalldata = _execCalldata(address(usdc), spend, address(weth));
        bytes memory gasCalldata = _execCalldata(address(usdc), GAS_SELL, address(0));
        intent.routerCalldataHash = keccak256(strategyCalldata);
        intent.gasRouterCalldataHash = keccak256(gasCalldata);

        vm.prank(gasRecipient);
        wallet.executeSwapWithFees(intent, strategyCalldata, gasCalldata, _signBundleIntent(intent));

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.deployedUsdc, spend);
        assertEq(session.capacityUsdc, LIMIT_USDC - PLATFORM_FEE - GAS_SELL);
        assertEq(uint256(session.capacityUsdc) - uint256(session.deployedUsdc), 0);
    }

    function testBuyWithFeesSpendLimitExceededWhenAllInExceeded() public {
        uint256 spend = LIMIT_USDC - PLATFORM_FEE - GAS_SELL + 1;
        _approveForHolder(address(usdc), spend);

        SessionSpend7702.SwapBundleIntent memory intent = _buyBundleIntent(
            spend, 1, wallet.sessionOf(STRATEGY_A, sessionKey).nonce, PLATFORM_FEE, GAS_SELL, 1
        );
        bytes memory strategyCalldata = _execCalldata(address(usdc), spend, address(weth));
        bytes memory gasCalldata = _execCalldata(address(usdc), GAS_SELL, address(0));
        intent.routerCalldataHash = keccak256(strategyCalldata);
        intent.gasRouterCalldataHash = keccak256(gasCalldata);

        vm.expectRevert(SessionSpend7702.SpendLimitExceeded.selector);
        vm.prank(gasRecipient);
        wallet.executeSwapWithFees(intent, strategyCalldata, gasCalldata, _signBundleIntent(intent));
    }

    function testBuyWithFeesDoesNotIncreaseDeployedForFees() public {
        uint256 spend = 100_000_000;
        _buyWithFees(spend, 0.09 ether, PLATFORM_FEE, GAS_SELL, 1);

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.deployedUsdc, spend);
        assertLt(session.capacityUsdc, LIMIT_USDC);
    }

    // ── executeSwapWithFees sell ─────────────────────────────────────────────

    function testSellWithFeesExecutesStrategyBeforeChargingFees() public {
        _buy(200_000_000, 0.2 ether);
        uint256 treasuryBefore = usdc.balanceOf(feeRecipient);
        uint256 gasBefore = gasRecipient.balance;

        _sellWithFees(0.2 ether, 1, PLATFORM_FEE, GAS_SELL, 1);

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.deployedUsdc, 0);
        assertEq(session.capacityUsdc, LIMIT_USDC - PLATFORM_FEE - GAS_SELL);
        assertEq(usdc.balanceOf(feeRecipient), treasuryBefore + PLATFORM_FEE);
        assertGt(gasRecipient.balance, gasBefore);
    }

    function testSellWithFeesDeductsCapacityNotDeployed() public {
        _buy(200_000_000, 0.2 ether);
        _sellWithFees(0.2 ether, 1, PLATFORM_FEE, GAS_SELL, 1);

        SessionSpend7702.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.deployedUsdc, 0);
        assertEq(session.capacityUsdc, LIMIT_USDC - PLATFORM_FEE - GAS_SELL);
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    function _buyWithFees(
        uint256 spend,
        uint256 minBuy,
        uint256 platformFee,
        uint256 gasSell,
        uint256 minNativeOut
    ) internal {
        SessionSpend7702.SwapBundleIntent memory intent = _buyBundleIntent(
            spend,
            minBuy,
            wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
            platformFee,
            gasSell,
            minNativeOut
        );
        _swapBundle(sessionKeyPrivateKey, intent);
    }

    function _sellWithFees(
        uint256 sellAmount,
        uint256 minBuyUsdc,
        uint256 platformFee,
        uint256 gasSell,
        uint256 minNativeOut
    ) internal {
        SessionSpend7702.SwapBundleIntent memory intent =
            SessionSpend7702.SwapBundleIntent({
                strategyId: STRATEGY_A,
                sessionKey: sessionKey,
                nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
                deadline: EXPIRES_AT,
                sellToken: address(weth),
                buyToken: address(usdc),
                maxSellAmount: sellAmount,
                minBuyAmount: minBuyUsdc,
                routerCalldataHash: bytes32(0),
                platformFeeUsdc: platformFee,
                feeRecipient: feeRecipient,
                gasSellUsdc: gasSell,
                minNativeOut: minNativeOut,
                gasRecipient: gasRecipient,
                gasRouterCalldataHash: bytes32(0)
            });
        _swapBundle(sessionKeyPrivateKey, intent);
    }

    function _swapBundle(uint256 privateKey, SessionSpend7702.SwapBundleIntent memory intent)
        internal
    {
        bytes memory strategyCalldata =
            _execCalldata(intent.sellToken, intent.maxSellAmount, intent.buyToken);
        bytes memory gasCalldata = intent.gasSellUsdc > 0
            ? _execCalldata(address(usdc), intent.gasSellUsdc, address(0))
            : bytes("");
        intent.routerCalldataHash = keccak256(strategyCalldata);
        intent.gasRouterCalldataHash = intent.gasSellUsdc > 0 ? keccak256(gasCalldata) : bytes32(0);
        vm.prank(intent.gasRecipient);
        wallet.executeSwapWithFees(
            intent, strategyCalldata, gasCalldata, _signBundleIntentWithKey(privateKey, intent)
        );
    }

    function _buyBundleIntent(
        uint256 maxSell,
        uint256 minBuy,
        uint256 nonce,
        uint256 platformFee,
        uint256 gasSell,
        uint256 minNativeOut
    ) internal view returns (SessionSpend7702.SwapBundleIntent memory) {
        return SessionSpend7702.SwapBundleIntent({
                strategyId: STRATEGY_A,
                sessionKey: sessionKey,
                nonce: nonce,
                deadline: EXPIRES_AT,
                sellToken: address(usdc),
                buyToken: address(weth),
                maxSellAmount: maxSell,
                minBuyAmount: minBuy,
                routerCalldataHash: bytes32(0),
                platformFeeUsdc: platformFee,
                feeRecipient: feeRecipient,
                gasSellUsdc: gasSell,
                minNativeOut: minNativeOut,
                gasRecipient: gasRecipient,
                gasRouterCalldataHash: bytes32(0)
            });
    }

    function _signBundleIntent(SessionSpend7702.SwapBundleIntent memory intent)
        internal
        view
        returns (bytes memory)
    {
        return _signBundleIntentWithKey(sessionKeyPrivateKey, intent);
    }

    function _signBundleIntentWithKey(
        uint256 privateKey,
        SessionSpend7702.SwapBundleIntent memory intent
    ) internal view returns (bytes memory) {
        bytes32 coreHash = keccak256(
            abi.encode(
                keccak256(
                    "SwapBundleCore(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,address sellToken,address buyToken,uint256 maxSellAmount,uint256 minBuyAmount,bytes32 routerCalldataHash)"
                ),
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
                keccak256(
                    "SwapBundleFees(uint256 platformFeeUsdc,address feeRecipient,uint256 gasSellUsdc,uint256 minNativeOut,address gasRecipient,bytes32 gasRouterCalldataHash)"
                ),
                intent.platformFeeUsdc,
                intent.feeRecipient,
                intent.gasSellUsdc,
                intent.minNativeOut,
                intent.gasRecipient,
                intent.gasRouterCalldataHash
            )
        );
        bytes32 digest = keccak256(
            abi.encodePacked(
                "\x19\x01",
                _domainSeparator(),
                keccak256(
                    abi.encode(
                        keccak256(
                            "SwapBundleIntent(SwapBundleCore core,SwapBundleFees fees)SwapBundleCore(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,address sellToken,address buyToken,uint256 maxSellAmount,uint256 minBuyAmount,bytes32 routerCalldataHash)SwapBundleFees(uint256 platformFeeUsdc,address feeRecipient,uint256 gasSellUsdc,uint256 minNativeOut,address gasRecipient,bytes32 gasRouterCalldataHash)"
                        ),
                        coreHash,
                        feesHash
                    )
                )
            )
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(privateKey, digest);
        return abi.encodePacked(r, s, v);
    }

    function _expectSwapReverts(bytes4 selector, uint256 nonce) internal {
        _approveForHolder(address(usdc), 10_000_000);
        SessionSpend7702.SwapIntent memory intent = _buyIntent(10_000_000, 0.009 ether, nonce);
        bytes memory routerCalldata = _execCalldata(address(usdc), 10_000_000, address(weth));
        intent.routerCalldataHash = keccak256(routerCalldata);

        vm.expectRevert(selector);
        wallet.executeSwap(intent, routerCalldata, _signIntent(intent));
    }

    function _setRate(address sellToken, address buyToken, uint256 numerator) internal {
        (bool ok,) = ALLOWANCE_HOLDER.call(
            abi.encodeWithSelector(holder.setRate.selector, sellToken, buyToken, numerator)
        );
        require(ok, "setRate");
    }

    function _setAttack(bool pullExtra, bool revertAfterPull) internal {
        (bool ok,) = ALLOWANCE_HOLDER.call(
            abi.encodeWithSelector(holder.setAttack.selector, pullExtra, revertAfterPull)
        );
        require(ok, "setAttack");
    }

    function _observedAllowance(address token, address owner) internal view returns (uint256) {
        (bool ok, bytes memory result) = ALLOWANCE_HOLDER.staticcall(
            abi.encodeWithSelector(holder.observedAllowance.selector, token, owner)
        );
        require(ok, "observedAllowance");
        return abi.decode(result, (uint256));
    }

    function _buy(uint256 spend, uint256 minBuy) internal {
        _swap(
            sessionKeyPrivateKey,
            _buyIntent(spend, minBuy, wallet.sessionOf(STRATEGY_A, sessionKey).nonce)
        );
    }

    function _sell(uint256 sellAmount, uint256 minBuyUsdc) internal {
        SessionSpend7702.SwapIntent memory intent = SessionSpend7702.SwapIntent({
            strategyId: STRATEGY_A,
            sessionKey: sessionKey,
            nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
            deadline: EXPIRES_AT,
            sellToken: address(weth),
            buyToken: address(usdc),
            maxSellAmount: sellAmount,
            minBuyAmount: minBuyUsdc,
            routerCalldataHash: bytes32(0)
        });
        _swap(sessionKeyPrivateKey, intent);
    }

    function _swap(uint256 privateKey, SessionSpend7702.SwapIntent memory intent) internal {
        bytes memory routerCalldata =
            _execCalldata(intent.sellToken, intent.maxSellAmount, intent.buyToken);
        intent.routerCalldataHash = keccak256(routerCalldata);
        wallet.executeSwap(intent, routerCalldata, _signIntentWithKey(privateKey, intent));
    }

    function _buyIntent(uint256 maxSell, uint256 minBuy, uint256 nonce)
        internal
        view
        returns (SessionSpend7702.SwapIntent memory)
    {
        return SessionSpend7702.SwapIntent({
            strategyId: STRATEGY_A,
            sessionKey: sessionKey,
            nonce: nonce,
            deadline: EXPIRES_AT,
            sellToken: address(usdc),
            buyToken: address(weth),
            maxSellAmount: maxSell,
            minBuyAmount: minBuy,
            routerCalldataHash: bytes32(0)
        });
    }

    function _execCalldata(address sellToken, uint256 amount, address buyToken)
        internal
        pure
        returns (bytes memory)
    {
        return abi.encodeWithSelector(
            0x2213bc0b,
            address(0x1111),
            sellToken,
            amount,
            address(0x1111),
            abi.encode(bytes4(0x12345678), buyToken)
        );
    }

    function _approveForHolder(address token, uint256 amount) internal {
        vm.prank(address(wallet));
        MockERC20(token).approve(ALLOWANCE_HOLDER, amount);
    }

    function _signIntent(SessionSpend7702.SwapIntent memory intent)
        internal
        view
        returns (bytes memory)
    {
        return _signIntentWithKey(sessionKeyPrivateKey, intent);
    }

    function _signIntentWithKey(uint256 privateKey, SessionSpend7702.SwapIntent memory intent)
        internal
        view
        returns (bytes memory)
    {
        bytes32 digest = keccak256(
            abi.encodePacked(
                "\x19\x01",
                _domainSeparator(),
                keccak256(
                    abi.encode(
                        keccak256(
                            "SwapIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,address sellToken,address buyToken,uint256 maxSellAmount,uint256 minBuyAmount,bytes32 routerCalldataHash)"
                        ),
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
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(privateKey, digest);
        return abi.encodePacked(r, s, v);
    }

    function _signRevokeIntent(SessionSpend7702.RevokeIntent memory intent)
        internal
        view
        returns (bytes memory)
    {
        bytes32 digest = keccak256(
            abi.encodePacked(
                "\x19\x01",
                _domainSeparator(),
                keccak256(
                    abi.encode(
                        keccak256(
                            "RevokeIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline)"
                        ),
                        intent.strategyId,
                        intent.sessionKey,
                        intent.nonce,
                        intent.deadline
                    )
                )
            )
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(sessionKeyPrivateKey, digest);
        return abi.encodePacked(r, s, v);
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
                address(wallet)
            )
        );
    }
}
