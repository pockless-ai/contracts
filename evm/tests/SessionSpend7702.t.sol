// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {SessionSpend7702} from "../src/SessionSpend7702.sol";
import {SessionSpendBase} from "../src/SessionSpendBase.sol";
import {StrategyVault} from "../src/StrategyVault.sol";

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

contract MockAllowanceHolder {
    address internal constant NATIVE_SENTINEL = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;

    mapping(address => mapping(address => uint256)) public rateNumerator;
    mapping(address => mapping(address => uint256)) public observedAllowance;
    uint256 public observedCallValue;

    function setRate(address sellToken, address buyToken, uint256 numerator) external {
        rateNumerator[sellToken][buyToken] = numerator;
    }

    function exec(address, address token, uint256 amount, address, bytes calldata data)
        external
        payable
        returns (bytes memory)
    {
        (, address buyToken) = abi.decode(data, (bytes4, address));
        bool nativeSell = token == address(0) || token == NATIVE_SENTINEL;
        address rateToken = nativeSell ? address(0) : token;
        uint256 numerator = rateNumerator[rateToken][buyToken];
        require(numerator > 0, "rate");
        observedCallValue = msg.value;
        if (nativeSell) {
            require(msg.value == amount, "native value");
        } else {
            require(msg.value == 0, "unexpected value");
            observedAllowance[token][msg.sender] =
                MockERC20(token).allowance(msg.sender, address(this));
            MockERC20(token).transferFrom(msg.sender, address(this), amount);
        }
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
}

contract MockRelayDepository {
    function pullFrom(address from, address token, uint256 amount) external {
        MockERC20(token).transferFrom(from, address(this), amount);
    }

    receive() external payable {}
}

contract SessionSpend7702Test is Test {
    address internal constant ALLOWANCE_HOLDER = 0x0000000000001fF3684f28c67538d4D072C22734;
    address internal constant RELAY_DEPOSITORY = 0x4cD00E387622C35bDDB9b4c962C136462338BC31;
    address internal constant NATIVE_SENTINEL = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;

    SessionSpend7702 internal wallet;
    MockERC20 internal usdc;
    MockERC20 internal weth;
    MockAllowanceHolder internal holder;
    MockRelayDepository internal relay;

    bytes32 internal constant STRATEGY_A = keccak256("strategy-a");
    bytes32 internal constant STRATEGY_B = keccak256("strategy-b");
    bytes32 internal constant RELAY_ORDER_A = keccak256("relay-order-a");
    bytes32 internal constant RELAY_ORDER_B = keccak256("relay-order-b");

    uint256 internal sessionKeyPrivateKey = 0xA11CE;
    address internal sessionKey;
    address internal platformRelayer = address(uint160(0x0E1A));

    uint256 internal constant LIMIT_USDC = 1_000_000_000;
    uint256 internal constant EXPIRES_AT = 4_102_444_800;
    uint256 internal constant FUNDING_CHAIN_ID = 8453;

    address internal feeRecipient = address(0xFEE);
    address internal gasRecipient = address(0x600D);
    uint256 internal constant PLATFORM_FEE = 1_000_000;
    uint256 internal constant NATIVE_RATE = 1e27;

    function setUp() public {
        sessionKey = vm.addr(sessionKeyPrivateKey);

        usdc = new MockERC20("USD Coin", "USDC", 6);
        weth = new MockERC20("Wrapped Ether", "WETH", 18);
        holder = new MockAllowanceHolder();
        relay = new MockRelayDepository();

        vm.etch(ALLOWANCE_HOLDER, address(holder).code);
        vm.etch(RELAY_DEPOSITORY, address(relay).code);
        _setRate(address(usdc), address(weth), 1e27);
        _setRate(address(weth), address(usdc), 1e9);
        _setRate(address(usdc), address(0), NATIVE_RATE);
        _setRate(address(0), address(usdc), 1e9);

        SessionSpend7702 implementation = new SessionSpend7702(address(usdc));
        address delegatedEoa = vm.addr(0x7702);
        vm.etch(delegatedEoa, address(implementation).code);
        wallet = SessionSpend7702(payable(delegatedEoa));

        usdc.mint(address(wallet), 10_000_000_000);
        usdc.mint(ALLOWANCE_HOLDER, 10_000_000_000);
        weth.mint(ALLOWANCE_HOLDER, 1_000 ether);
        vm.deal(ALLOWANCE_HOLDER, 1_000 ether);

        vm.startPrank(address(wallet));
        wallet.setPlatformRelayer(platformRelayer);
        wallet.grant(STRATEGY_A, sessionKey, LIMIT_USDC, EXPIRES_AT);
        vm.stopPrank();
    }

    // ── grant / vault ────────────────────────────────────────────────────────

    function testDelegatedRuntimeFitsEip170() public view {
        assertLe(address(wallet).code.length, 24_576);
    }

    function testGrantDeploysDeterministicStrategyVault() public view {
        address vault = wallet.strategyVaultOf(STRATEGY_A);
        assertEq(vault, wallet.predictStrategyVault(STRATEGY_A));
        assertEq(StrategyVault(payable(vault)).owner(), address(wallet));
    }

    function testGrantInitializesSessionState() public view {
        SessionSpendBase.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertTrue(session.exists);
        assertFalse(session.revoked);
        assertEq(session.limitUsdc, LIMIT_USDC);
        assertEq(session.capacityUsdc, LIMIT_USDC);
        assertEq(session.deployedUsdc, 0);
        assertEq(session.nonce, 0);
    }

    function testGrantRejectsExistingSession() public {
        vm.prank(address(wallet));
        vm.expectRevert(SessionSpendBase.SessionAlreadyExists.selector);
        wallet.grant(STRATEGY_A, sessionKey, LIMIT_USDC, EXPIRES_AT);
    }

    // ── revoke ─────────────────────────────────────────────────────────────

    function testRevokeBySessionKeyIncrementsNonce() public {
        vm.prank(sessionKey);
        wallet.revoke(STRATEGY_A, sessionKey);

        SessionSpendBase.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertTrue(session.revoked);
        assertEq(session.nonce, 1);
    }

    function testRevokedSessionRejectsV2Swap() public {
        vm.prank(sessionKey);
        wallet.revoke(STRATEGY_A, sessionKey);
        _expectV2SwapReverts(SessionSpendBase.SessionRevoked.selector);
    }

    // ── executeSwapWithFeesV2 ───────────────────────────────────────────────

    function testV2CreditOnlyExecutesWithoutTopUp() public {
        SessionSpendBase.SwapBundleIntentV2 memory intent = _buyBundleIntentV2(
            100_000_000, 0.09 ether, 0, SessionSpendBase.GasFundingMode.CREDIT_ONLY, 0, 0
        );
        _swapBundleV2(intent, address(this));
        assertEq(wallet.sessionOf(STRATEGY_A, sessionKey).deployedUsdc, 100_000_000);
    }

    function testV2SeparateTopUpTransfersAndEmitsAfterSuccess() public {
        SessionSpendBase.SwapBundleIntentV2 memory intent = _buyBundleIntentV2(
            100_000_000,
            0.09 ether,
            0,
            SessionSpendBase.GasFundingMode.SEPARATE_TOPUP,
            GAS_TOP_UP_USDC(),
            GAS_TOP_UP_NATIVE()
        );
        uint256 before = gasRecipient.balance;
        _swapBundleV2(intent, gasRecipient);
        assertGt(gasRecipient.balance, before);
    }

    function testV2SellsNativeInventoryBackToUsdc() public {
        SessionSpendBase.SwapBundleIntentV2 memory buy = _buyBundleIntentV2(
            100_000_000, 0.09 ether, 0, SessionSpendBase.GasFundingMode.CREDIT_ONLY, 0, 0
        );
        buy.buyToken = address(0);
        _swapBundleV2(buy, address(this));

        uint256 quantity = address(wallet).balance;
        SessionSpendBase.SwapBundleIntentV2 memory sell = _buyBundleIntentV2(
            quantity, 1, 0, SessionSpendBase.GasFundingMode.CREDIT_ONLY, 0, 0
        );
        sell.sellToken = address(0);
        sell.buyToken = address(usdc);
        uint256 usdcBefore = usdc.balanceOf(address(wallet));
        _swapBundleV2(sell, address(this));

        assertEq(address(wallet).balance, 0);
        assertGt(usdc.balanceOf(address(wallet)), usdcBefore);
    }

    function testV2UsesDomainVersionOne() public {
        SessionSpendBase.SwapBundleIntentV2 memory intent = _buyBundleIntentV2(
            100_000_000, 0.09 ether, 0, SessionSpendBase.GasFundingMode.CREDIT_ONLY, 0, 0
        );
        bytes memory strategyCalldata =
            _execCalldata(address(usdc), intent.strategySellAmount, address(weth));
        intent.strategyRouterCalldataHash = keccak256(strategyCalldata);
        bytes memory badSig = _signBundleIntentV2WithVersion(intent, "2");
        vm.expectRevert(SessionSpendBase.InvalidSignature.selector);
        wallet.executeSwapWithFeesV2(intent, strategyCalldata, "", badSig);
    }

    // ── relay deposit ───────────────────────────────────────────────────────

    function testRelayDepositLocksCapacityAndSpendsFromVault() public {
        uint256 originAmount = 100_000_000;
        address vault = wallet.strategyVaultOf(STRATEGY_A);
        uint256 vaultBefore = usdc.balanceOf(vault);

        _executeRelayDeposit(RELAY_ORDER_A, originAmount, 0, vault);

        SessionSpendBase.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.deployedUsdc, originAmount);
        assertEq(session.nonce, 1);
        assertTrue(
            wallet.relayReceiptConsumed(
                STRATEGY_A, RELAY_ORDER_A, SessionSpendBase.RelayAction.Deposit
            )
        );

        SessionSpendBase.PendingDeposit memory pending = wallet.pendingDepositOf(RELAY_ORDER_A);
        assertTrue(pending.exists);
        assertEq(pending.lockedCostUsdc, uint128(originAmount));

        assertEq(usdc.balanceOf(vault), vaultBefore);
        assertEq(usdc.balanceOf(RELAY_DEPOSITORY), originAmount);
    }

    function testRelayDepositRejectsWrongRelayer() public {
        SessionSpendBase.RelayDepositIntent memory intent =
            _relayDepositIntent(RELAY_ORDER_A, 100_000_000);
        (address target, bytes memory data, bytes32 callHash) =
            _relayCall(vaultFor(STRATEGY_A), address(usdc), 100_000_000);
        intent.relayCalldataHash = callHash;
        bytes memory sig = _signRelayDeposit(intent);

        vm.prank(address(0xBAD));
        vm.expectRevert(SessionSpendBase.NotPlatformRelayer.selector);
        wallet.executeRelayDeposit(intent, target, data, 0, sig);
    }

    function testRelayDepositReplayRejected() public {
        _executeRelayDeposit(RELAY_ORDER_A, 50_000_000, 0, wallet.strategyVaultOf(STRATEGY_A));

        SessionSpendBase.RelayDepositIntent memory intent =
            _relayDepositIntent(RELAY_ORDER_A, 50_000_000);
        intent.nonce = 1;
        (address target, bytes memory data, bytes32 callHash) =
            _relayCall(vaultFor(STRATEGY_A), address(usdc), 50_000_000);
        intent.relayCalldataHash = callHash;

        vm.prank(platformRelayer);
        vm.expectRevert(SessionSpendBase.RelayReceiptConsumed.selector);
        wallet.executeRelayDeposit(intent, target, data, 0, _signRelayDeposit(intent));
    }

    function testRelayDepositRequiresRefundVaultMatchStrategyVault() public {
        SessionSpendBase.RelayDepositIntent memory intent =
            _relayDepositIntent(RELAY_ORDER_A, 100_000_000);
        intent.refundVault = address(0xBEEF);
        (address target, bytes memory data, bytes32 callHash) =
            _relayCall(vaultFor(STRATEGY_A), address(usdc), 100_000_000);
        intent.relayCalldataHash = callHash;

        vm.prank(platformRelayer);
        vm.expectRevert(SessionSpendBase.InvalidIntent.selector);
        wallet.executeRelayDeposit(intent, target, data, 0, _signRelayDeposit(intent));
    }

    // ── credit relay asset ──────────────────────────────────────────────────

    function testCreditRelayAssetUpdatesRemoteInventoryAndAccountedBalance() public {
        _executeRelayDeposit(RELAY_ORDER_B, 100_000_000, 0, wallet.strategyVaultOf(STRATEGY_A));
        address vault = wallet.strategyVaultOf(STRATEGY_A);
        weth.mint(vault, 1 ether);

        _creditRelayAsset(RELAY_ORDER_B, address(weth), 1 ether, 100_000_000);

        SessionSpendBase.RemoteAssetRecord memory remote =
            wallet.remoteAssetOf(STRATEGY_A, address(weth), FUNDING_CHAIN_ID);
        assertEq(remote.quantity, 1 ether);
        assertEq(remote.costUsdc, 100_000_000);
        assertEq(wallet.vaultAccountedBalanceOf(STRATEGY_A, address(weth)), 1 ether);
    }

    function testCreditRelayAssetRejectsExcessCredit() public {
        _executeRelayDeposit(RELAY_ORDER_B, 100_000_000, 0, wallet.strategyVaultOf(STRATEGY_A));
        address vault = wallet.strategyVaultOf(STRATEGY_A);
        weth.mint(vault, 0.5 ether);

        SessionSpendBase.CreditRelayAssetIntent memory intent =
            SessionSpendBase.CreditRelayAssetIntent({
                strategyId: STRATEGY_A,
                sessionKey: sessionKey,
                nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
                deadline: EXPIRES_AT,
                relayOrderId: RELAY_ORDER_B,
                token: address(weth),
                fundingChainId: FUNDING_CHAIN_ID,
                creditQuantity: 1 ether,
                costUsdc: 100_000_000
            });

        vm.prank(platformRelayer);
        vm.expectRevert(SessionSpendBase.InsufficientVaultSurplus.selector);
        wallet.creditRelayAsset(intent, _signCreditRelayAsset(intent));
    }

    // ── remote relay sell + return ──────────────────────────────────────────

    function testRemoteRelaySellConsumesRemoteInventory() public {
        _seedRemoteInventory(1 ether, 200_000_000);
        _executeRemoteRelaySell(RELAY_ORDER_B, 0.5 ether);

        SessionSpendBase.RemoteAssetRecord memory remote =
            wallet.remoteAssetOf(STRATEGY_A, address(weth), FUNDING_CHAIN_ID);
        assertEq(remote.quantity, 0.5 ether);
        assertEq(remote.costUsdc, 100_000_000);

        SessionSpendBase.PendingSell memory pending = wallet.pendingSellOf(RELAY_ORDER_B);
        assertTrue(pending.exists);
        assertEq(pending.quantity, 0.5 ether);
        assertEq(pending.provisionalCostUsdc, 100_000_000);
    }

    function testCreditUsdcReturnReleasesDeployedAndAppliesProfit() public {
        _seedRemoteInventory(1 ether, 200_000_000);
        _executeRemoteRelaySell(RELAY_ORDER_B, 0.5 ether);

        address vault = wallet.strategyVaultOf(STRATEGY_A);
        usdc.mint(vault, 220_000_000);
        uint256 ownerBefore = usdc.balanceOf(address(wallet));

        SessionSpendBase.CreditUsdcReturnIntent memory intent =
            SessionSpendBase.CreditUsdcReturnIntent({
                strategyId: STRATEGY_A,
                sessionKey: sessionKey,
                nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
                deadline: EXPIRES_AT,
                relayOrderId: RELAY_ORDER_B,
                fundingChainId: FUNDING_CHAIN_ID,
                usdcReceived: 220_000_000,
                destQuantityReleased: 0.5 ether,
                destCostReleasedUsdc: 100_000_000,
                platformFeeUsdc: 0,
                feeRecipient: address(0)
            });

        vm.prank(platformRelayer);
        wallet.creditUsdcReturn(intent, _signCreditUsdcReturn(intent));

        SessionSpendBase.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.deployedUsdc, 100_000_000);
        assertEq(session.capacityUsdc, LIMIT_USDC);
        assertFalse(wallet.pendingSellOf(RELAY_ORDER_B).exists);
        assertEq(usdc.balanceOf(vault), 0);
        assertEq(usdc.balanceOf(address(wallet)), ownerBefore + 220_000_000);
    }

    function testCreditUsdcReturnSendsNetProceedsToOwnerAndFeeToTreasury() public {
        _seedRemoteInventory(1 ether, 200_000_000);
        _executeRemoteRelaySell(RELAY_ORDER_B, 0.5 ether);

        address vault = wallet.strategyVaultOf(STRATEGY_A);
        usdc.mint(vault, 220_000_000);
        uint256 ownerBefore = usdc.balanceOf(address(wallet));
        uint256 feeBefore = usdc.balanceOf(feeRecipient);

        SessionSpendBase.CreditUsdcReturnIntent memory intent =
            SessionSpendBase.CreditUsdcReturnIntent({
                strategyId: STRATEGY_A,
                sessionKey: sessionKey,
                nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
                deadline: EXPIRES_AT,
                relayOrderId: RELAY_ORDER_B,
                fundingChainId: FUNDING_CHAIN_ID,
                usdcReceived: 220_000_000,
                destQuantityReleased: 0.5 ether,
                destCostReleasedUsdc: 100_000_000,
                platformFeeUsdc: PLATFORM_FEE,
                feeRecipient: feeRecipient
            });

        vm.prank(platformRelayer);
        wallet.creditUsdcReturn(intent, _signCreditUsdcReturn(intent));

        assertEq(usdc.balanceOf(feeRecipient), feeBefore + PLATFORM_FEE);
        assertEq(usdc.balanceOf(address(wallet)), ownerBefore + 220_000_000 - PLATFORM_FEE);
        assertEq(usdc.balanceOf(vault), 0);
    }

    function testCreditUsdcReturnRequiresPendingSell() public {
        _seedRemoteInventory(1 ether, 200_000_000);
        address vault = wallet.strategyVaultOf(STRATEGY_A);
        usdc.mint(vault, 220_000_000);

        SessionSpendBase.CreditUsdcReturnIntent memory intent =
            SessionSpendBase.CreditUsdcReturnIntent({
                strategyId: STRATEGY_A,
                sessionKey: sessionKey,
                nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
                deadline: EXPIRES_AT,
                relayOrderId: RELAY_ORDER_B,
                fundingChainId: FUNDING_CHAIN_ID,
                usdcReceived: 220_000_000,
                destQuantityReleased: 0.5 ether,
                destCostReleasedUsdc: 100_000_000,
                platformFeeUsdc: 0,
                feeRecipient: address(0)
            });

        vm.prank(platformRelayer);
        vm.expectRevert(SessionSpendBase.PendingRecordMissing.selector);
        wallet.creditUsdcReturn(intent, _signCreditUsdcReturn(intent));
    }

    function testCreditUsdcReturnOpensFundingPendingWhenOriginPendingIsOnAnotherChain() public {
        _seedRemoteInventory(1 ether, 200_000_000);
        address vault = wallet.strategyVaultOf(STRATEGY_A);
        usdc.mint(vault, 220_000_000);
        uint256 ownerBefore = usdc.balanceOf(address(wallet));

        SessionSpendBase.CreditUsdcReturnIntent memory intent =
            SessionSpendBase.CreditUsdcReturnIntent({
                strategyId: STRATEGY_A,
                sessionKey: sessionKey,
                nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
                deadline: EXPIRES_AT,
                relayOrderId: RELAY_ORDER_B,
                fundingChainId: block.chainid,
                usdcReceived: 220_000_000,
                destQuantityReleased: 1 ether,
                destCostReleasedUsdc: 200_000_000,
                platformFeeUsdc: 0,
                feeRecipient: address(0)
            });

        vm.prank(platformRelayer);
        wallet.creditUsdcReturn(intent, _signCreditUsdcReturn(intent));

        SessionSpendBase.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.deployedUsdc, 0);
        assertFalse(wallet.pendingSellOf(RELAY_ORDER_B).exists);
        assertEq(usdc.balanceOf(vault), 0);
        assertEq(usdc.balanceOf(address(wallet)), ownerBefore + 220_000_000);
    }

    function testReleaseRelayDepositReversesDeployedLock() public {
        _executeRelayDeposit(RELAY_ORDER_A, 150_000_000, 0, wallet.strategyVaultOf(STRATEGY_A));

        address vault = wallet.strategyVaultOf(STRATEGY_A);
        usdc.mint(vault, 150_000_000);

        SessionSpendBase.ReleaseRelayDepositIntent memory intent =
            SessionSpendBase.ReleaseRelayDepositIntent({
                strategyId: STRATEGY_A,
                sessionKey: sessionKey,
                nonce: 1,
                deadline: EXPIRES_AT,
                relayOrderId: RELAY_ORDER_A,
                token: address(usdc),
                fundingChainId: FUNDING_CHAIN_ID,
                refundQuantity: 150_000_000,
                refundCostUsdc: 150_000_000
            });

        vm.prank(platformRelayer);
        wallet.releaseRelayDeposit(intent, _signReleaseRelayDeposit(intent));

        SessionSpendBase.Session memory session = wallet.sessionOf(STRATEGY_A, sessionKey);
        assertEq(session.deployedUsdc, 0);
        assertFalse(wallet.pendingDepositOf(RELAY_ORDER_A).exists);
    }

    function testRestoreRemoteRelayAssetRestoresInventory() public {
        _seedRemoteInventory(1 ether, 200_000_000);
        _executeRemoteRelaySell(RELAY_ORDER_B, 1 ether);

        address vault = wallet.strategyVaultOf(STRATEGY_A);
        weth.mint(vault, 1 ether);

        SessionSpendBase.RestoreRemoteRelayAssetIntent memory intent =
            SessionSpendBase.RestoreRemoteRelayAssetIntent({
                strategyId: STRATEGY_A,
                sessionKey: sessionKey,
                nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
                deadline: EXPIRES_AT,
                relayOrderId: RELAY_ORDER_B,
                token: address(weth),
                fundingChainId: FUNDING_CHAIN_ID,
                restoreQuantity: 1 ether,
                restoreCostUsdc: 200_000_000
            });

        vm.prank(platformRelayer);
        wallet.restoreRemoteRelayAsset(intent, _signRestoreRemoteRelayAsset(intent));

        SessionSpendBase.RemoteAssetRecord memory remote =
            wallet.remoteAssetOf(STRATEGY_A, address(weth), FUNDING_CHAIN_ID);
        assertEq(remote.quantity, 1 ether);
        assertEq(remote.costUsdc, 200_000_000);
        assertFalse(wallet.pendingSellOf(RELAY_ORDER_B).exists);
    }

    // ── owner recovery ──────────────────────────────────────────────────────

    function testRecoverVaultSurplusTransfersUnaccountedBalance() public {
        address vault = wallet.strategyVaultOf(STRATEGY_A);
        usdc.mint(vault, 25_000_000);
        address recipient = address(0xCAFE);

        vm.prank(address(wallet));
        wallet.recoverVaultSurplus(STRATEGY_A, address(usdc), recipient, 25_000_000);

        assertEq(usdc.balanceOf(recipient), 25_000_000);
        assertEq(usdc.balanceOf(vault), 0);
    }

    function testRecoverVaultSurplusCannotTakeAccountedInventory() public {
        _seedRemoteInventory(1 ether, 100_000_000);

        vm.prank(address(wallet));
        vm.expectRevert(SessionSpendBase.InsufficientVaultSurplus.selector);
        wallet.recoverVaultSurplus(STRATEGY_A, address(weth), address(0xCAFE), 1 ether);
    }

    // ── helpers ─────────────────────────────────────────────────────────────

    function GAS_TOP_UP_USDC() internal pure returns (uint256) {
        return 500_000;
    }

    function GAS_TOP_UP_NATIVE() internal pure returns (uint256) {
        return 0.0005 ether;
    }

    function vaultFor(bytes32 strategyId) internal view returns (address) {
        return wallet.strategyVaultOf(strategyId);
    }

    function _seedRemoteInventory(uint256 quantity, uint128 costUsdc) internal {
        _executeRelayDeposit(RELAY_ORDER_A, costUsdc, 0, wallet.strategyVaultOf(STRATEGY_A));
        address vault = wallet.strategyVaultOf(STRATEGY_A);
        weth.mint(vault, quantity);
        _creditRelayAsset(RELAY_ORDER_A, address(weth), quantity, costUsdc);
    }

    function _executeRelayDeposit(
        bytes32 relayOrderId,
        uint256 originAmount,
        uint256 platformFee,
        address refundVault
    ) internal {
        SessionSpendBase.RelayDepositIntent memory intent =
            _relayDepositIntent(relayOrderId, originAmount);
        intent.platformFeeUsdc = platformFee;
        intent.feeRecipient = platformFee > 0 ? feeRecipient : address(0);
        intent.refundVault = refundVault;
        (address target, bytes memory data, bytes32 callHash) =
            _relayCall(refundVault, address(usdc), originAmount);
        intent.relayCalldataHash = callHash;

        vm.prank(platformRelayer);
        wallet.executeRelayDeposit(intent, target, data, 0, _signRelayDeposit(intent));
    }

    function _relayDepositIntent(bytes32 relayOrderId, uint256 originAmount)
        internal
        view
        returns (SessionSpendBase.RelayDepositIntent memory)
    {
        address vault = wallet.strategyVaultOf(STRATEGY_A);
        return SessionSpendBase.RelayDepositIntent({
            strategyId: STRATEGY_A,
            sessionKey: sessionKey,
            nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
            deadline: EXPIRES_AT,
            relayOrderId: relayOrderId,
            fundingChainId: FUNDING_CHAIN_ID,
            originToken: address(usdc),
            originTokenDecimals: 6,
            destToken: address(weth),
            destTokenDecimals: 18,
            originAmount: originAmount,
            destChainId: 42161,
            minDestAmount: 0.09 ether,
            destRecipient: address(0xDE57),
            refundVault: vault,
            relayCalldataHash: bytes32(0),
            platformFeeUsdc: 0,
            feeRecipient: address(0)
        });
    }

    function _relayCall(address vault, address token, uint256 amount)
        internal
        view
        returns (address target, bytes memory data, bytes32 callHash)
    {
        target = RELAY_DEPOSITORY;
        data = abi.encodeWithSignature("pullFrom(address,address,uint256)", vault, token, amount);
        callHash = keccak256(abi.encode(target, uint256(0), data));
    }

    function _creditRelayAsset(
        bytes32 relayOrderId,
        address token,
        uint256 creditQuantity,
        uint128 costUsdc
    ) internal {
        SessionSpendBase.CreditRelayAssetIntent memory intent =
            SessionSpendBase.CreditRelayAssetIntent({
                strategyId: STRATEGY_A,
                sessionKey: sessionKey,
                nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
                deadline: EXPIRES_AT,
                relayOrderId: relayOrderId,
                token: token,
                fundingChainId: FUNDING_CHAIN_ID,
                creditQuantity: creditQuantity,
                costUsdc: costUsdc
            });
        vm.prank(platformRelayer);
        wallet.creditRelayAsset(intent, _signCreditRelayAsset(intent));
    }

    function _executeRemoteRelaySell(bytes32 relayOrderId, uint256 sellQuantity) internal {
        SessionSpendBase.RemoteRelaySellIntent memory intent = SessionSpendBase.RemoteRelaySellIntent({
            strategyId: STRATEGY_A,
            sessionKey: sessionKey,
            nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
            deadline: EXPIRES_AT,
            relayOrderId: relayOrderId,
            token: address(weth),
            fundingChainId: FUNDING_CHAIN_ID,
            sellQuantity: sellQuantity,
            minReturnUsdc: 1,
            relayCalldataHash: bytes32(0)
        });
        (address target, bytes memory data, bytes32 callHash) =
            _relayCall(wallet.strategyVaultOf(STRATEGY_A), address(weth), sellQuantity);
        intent.relayCalldataHash = callHash;

        vm.prank(platformRelayer);
        wallet.executeRemoteRelaySell(intent, target, data, 0, _signRemoteRelaySell(intent));
    }

    function _buyBundleIntentV2(
        uint256 strategySell,
        uint256 minBuy,
        uint256 platformFee,
        SessionSpendBase.GasFundingMode mode,
        uint256 gasTopUpUsdc,
        uint256 gasTopUpNative
    ) internal view returns (SessionSpendBase.SwapBundleIntentV2 memory) {
        return SessionSpendBase.SwapBundleIntentV2({
                strategyId: STRATEGY_A,
                sessionKey: sessionKey,
                nonce: wallet.sessionOf(STRATEGY_A, sessionKey).nonce,
                deadline: EXPIRES_AT,
                sellToken: address(usdc),
                buyToken: address(weth),
                strategySellAmount: strategySell,
                minStrategyBuyAmount: minBuy,
                strategyRouterCalldataHash: bytes32(0),
                platformFeeUsdc: platformFee,
                feeRecipient: platformFee > 0 ? feeRecipient : address(0),
                gasFundingMode: mode,
                gasTopUpUsdc: gasTopUpUsdc,
                gasTopUpNative: gasTopUpNative,
                gasRecipient: mode == SessionSpendBase.GasFundingMode.CREDIT_ONLY
                    ? address(0)
                    : gasRecipient,
                gasRouterCalldataHash: bytes32(0)
            });
    }

    function _swapBundleV2(SessionSpendBase.SwapBundleIntentV2 memory intent, address broadcaster)
        internal
    {
        if (intent.gasFundingMode != SessionSpendBase.GasFundingMode.CREDIT_ONLY) {
            intent.gasRecipient = broadcaster;
        }
        bytes memory strategyCalldata =
            _execCalldata(intent.sellToken, intent.strategySellAmount, intent.buyToken);
        bytes memory gasCalldata = intent.gasFundingMode
            == SessionSpendBase.GasFundingMode.SEPARATE_TOPUP
            ? _execCalldata(address(usdc), intent.gasTopUpUsdc, address(0))
            : bytes("");
        intent.strategyRouterCalldataHash = keccak256(strategyCalldata);
        intent.gasRouterCalldataHash = gasCalldata.length > 0 ? keccak256(gasCalldata) : bytes32(0);
        vm.prank(broadcaster);
        wallet.executeSwapWithFeesV2(
            intent, strategyCalldata, gasCalldata, _signBundleIntentV2(intent)
        );
    }

    function _expectV2SwapReverts(bytes4 selector) internal {
        SessionSpendBase.SwapBundleIntentV2 memory intent = _buyBundleIntentV2(
            10_000_000, 0.009 ether, 0, SessionSpendBase.GasFundingMode.CREDIT_ONLY, 0, 0
        );
        bytes memory strategyCalldata =
            _execCalldata(address(usdc), intent.strategySellAmount, address(weth));
        intent.strategyRouterCalldataHash = keccak256(strategyCalldata);
        vm.expectRevert(selector);
        wallet.executeSwapWithFeesV2(intent, strategyCalldata, "", _signBundleIntentV2(intent));
    }

    function _execCalldata(address sellToken, uint256 amount, address buyToken)
        internal
        pure
        returns (bytes memory)
    {
        // 0x leaves the token slot empty for a native sell: there is no allowance to hold.
        return abi.encodeWithSelector(
            0x2213bc0b,
            address(0x1111),
            sellToken,
            amount,
            address(0x1111),
            abi.encode(bytes4(0x12345678), buyToken)
        );
    }

    function _setRate(address sellToken, address buyToken, uint256 numerator) internal {
        (bool ok,) = ALLOWANCE_HOLDER.call(
            abi.encodeWithSelector(holder.setRate.selector, sellToken, buyToken, numerator)
        );
        require(ok, "setRate");
    }

    function _signRelayDeposit(SessionSpendBase.RelayDepositIntent memory intent)
        internal
        view
        returns (bytes memory)
    {
        bytes32 typeHash = keccak256(
            "RelayDepositIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,uint256 fundingChainId,address originToken,uint8 originTokenDecimals,address destToken,uint8 destTokenDecimals,uint256 originAmount,uint256 destChainId,uint256 minDestAmount,address destRecipient,address refundVault,bytes32 relayCalldataHash,uint256 platformFeeUsdc,address feeRecipient)"
        );
        bytes memory firstHalf = abi.encode(
            typeHash,
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
        bytes32 digest = keccak256(
            abi.encodePacked(
                "\x19\x01", _domainSeparator(), keccak256(bytes.concat(firstHalf, secondHalf))
            )
        );
        return _signDigest(digest);
    }

    function _signCreditRelayAsset(SessionSpendBase.CreditRelayAssetIntent memory intent)
        internal
        view
        returns (bytes memory)
    {
        bytes32 typeHash = keccak256(
            "CreditRelayAssetIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,address token,uint256 fundingChainId,uint256 creditQuantity,uint128 costUsdc)"
        );
        bytes32 digest = keccak256(
            abi.encodePacked(
                "\x19\x01",
                _domainSeparator(),
                keccak256(
                    abi.encode(
                        typeHash,
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
        return _signDigest(digest);
    }

    function _signRemoteRelaySell(SessionSpendBase.RemoteRelaySellIntent memory intent)
        internal
        view
        returns (bytes memory)
    {
        bytes32 typeHash = keccak256(
            "RemoteRelaySellIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,address token,uint256 fundingChainId,uint256 sellQuantity,uint256 minReturnUsdc,bytes32 relayCalldataHash)"
        );
        bytes32 digest = keccak256(
            abi.encodePacked(
                "\x19\x01",
                _domainSeparator(),
                keccak256(
                    abi.encode(
                        typeHash,
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
        return _signDigest(digest);
    }

    function _signCreditUsdcReturn(SessionSpendBase.CreditUsdcReturnIntent memory intent)
        internal
        view
        returns (bytes memory)
    {
        bytes32 typeHash = keccak256(
            "CreditUsdcReturnIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,uint256 fundingChainId,uint256 usdcReceived,uint256 destQuantityReleased,uint128 destCostReleasedUsdc,uint256 platformFeeUsdc,address feeRecipient)"
        );
        bytes32 digest = keccak256(
            abi.encodePacked(
                "\x19\x01",
                _domainSeparator(),
                keccak256(
                    abi.encode(
                        typeHash,
                        intent.strategyId,
                        intent.sessionKey,
                        intent.nonce,
                        intent.deadline,
                        intent.relayOrderId,
                        intent.fundingChainId,
                        intent.usdcReceived,
                        intent.destQuantityReleased,
                        intent.destCostReleasedUsdc,
                        intent.platformFeeUsdc,
                        intent.feeRecipient
                    )
                )
            )
        );
        return _signDigest(digest);
    }

    function _signReleaseRelayDeposit(SessionSpendBase.ReleaseRelayDepositIntent memory intent)
        internal
        view
        returns (bytes memory)
    {
        bytes32 typeHash = keccak256(
            "ReleaseRelayDepositIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,address token,uint256 fundingChainId,uint256 refundQuantity,uint128 refundCostUsdc)"
        );
        bytes32 digest = keccak256(
            abi.encodePacked(
                "\x19\x01",
                _domainSeparator(),
                keccak256(
                    abi.encode(
                        typeHash,
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
        return _signDigest(digest);
    }

    function _signRestoreRemoteRelayAsset(
        SessionSpendBase.RestoreRemoteRelayAssetIntent memory intent
    ) internal view returns (bytes memory) {
        bytes32 typeHash = keccak256(
            "RestoreRemoteRelayAssetIntent(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,bytes32 relayOrderId,address token,uint256 fundingChainId,uint256 restoreQuantity,uint128 restoreCostUsdc)"
        );
        bytes32 digest = keccak256(
            abi.encodePacked(
                "\x19\x01",
                _domainSeparator(),
                keccak256(
                    abi.encode(
                        typeHash,
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
        return _signDigest(digest);
    }

    function _signBundleIntentV2(SessionSpendBase.SwapBundleIntentV2 memory intent)
        internal
        view
        returns (bytes memory)
    {
        return _signBundleIntentV2WithVersion(intent, "1");
    }

    function _signBundleIntentV2WithVersion(
        SessionSpendBase.SwapBundleIntentV2 memory intent,
        string memory version
    ) internal view returns (bytes memory) {
        bytes memory firstHalf = abi.encode(
            keccak256(
                "SwapBundleIntentV2(bytes32 strategyId,address sessionKey,uint256 nonce,uint256 deadline,address sellToken,address buyToken,uint256 strategySellAmount,uint256 minStrategyBuyAmount,bytes32 strategyRouterCalldataHash,uint256 platformFeeUsdc,address feeRecipient,uint8 gasFundingMode,uint256 gasTopUpUsdc,uint256 gasTopUpNative,address gasRecipient,bytes32 gasRouterCalldataHash)"
            ),
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
            uint8(intent.gasFundingMode),
            intent.gasTopUpUsdc,
            intent.gasTopUpNative,
            intent.gasRecipient,
            intent.gasRouterCalldataHash
        );
        bytes32 digest = keccak256(
            abi.encodePacked(
                "\x19\x01",
                _domainSeparator(version),
                keccak256(bytes.concat(firstHalf, secondHalf))
            )
        );
        return _signDigest(digest);
    }

    function _signDigest(bytes32 digest) internal view returns (bytes memory) {
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(sessionKeyPrivateKey, digest);
        return abi.encodePacked(r, s, v);
    }

    function _domainSeparator() internal view returns (bytes32) {
        return _domainSeparator("1");
    }

    function _domainSeparator(string memory version) internal view returns (bytes32) {
        return keccak256(
            abi.encode(
                keccak256(
                    "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
                ),
                keccak256(bytes("PocklessSessionSpend7702")),
                keccak256(bytes(version)),
                block.chainid,
                address(wallet)
            )
        );
    }
}
