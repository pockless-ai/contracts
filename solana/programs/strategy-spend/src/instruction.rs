use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GasMode {
    None,
    CreditOnly,
    Separate,
    NativeOutput,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum StrategySpendInstruction {
    /// Create the wallet config PDA. Owner must sign.
    InitWallet,
    /// Create a strategy PDA (create-only). Owner must sign.
    InitStrategy {
        strategy_id: [u8; 32],
        session: Pubkey,
        limit_usdc: u64,
        expires_at: i64,
    },
    /// Owner updates limit and expiry.
    SetLimit { limit_usdc: u64, expires_at: i64 },
    /// Owner rotates the session key and bumps nonce.
    RotateSession { new_session: Pubkey },
    /// Owner or session revokes the strategy.
    Revoke,
    /// Owner withdraws tokens from the strategy vault.
    WithdrawAsset { amount: u64 },
    /// Owner closes the strategy after all positions are flat.
    CloseStrategy,
    /// Session executes platform fee, gas handling, and one pinned Jupiter swap atomically.
    ExecuteSwapWithFees {
        is_buy: bool,
        usdc_amount: u64,
        token_amount: u64,
        platform_fee_usdc: u64,
        gas_mode: GasMode,
        gas_top_up_usdc: u64,
        native_amount: u64,
        treasury: Pubkey,
        gas_recipient: Pubkey,
        jupiter_data: Vec<u8>,
        gas_jupiter_data: Vec<u8>,
    },
    /// Origin-chain Relay deposit: lock deployed cost and CPI to Relay Depository.
    ExecuteRelayDeposit {
        relay_order_id: [u8; 32],
        funding_chain_id: u64,
        amount: u64,
        min_dest_amount: u64,
        locked_cost_usdc: u64,
        platform_fee_usdc: u64,
        nonce: u64,
        deadline: i64,
        relay_ix_data: Vec<u8>,
    },
    /// Destination-chain credit of Relay-delivered remote inventory.
    CreditRelayAsset {
        relay_order_id: [u8; 32],
        funding_chain_id: u64,
        credit_quantity: u64,
        cost_usdc: u64,
        min_credit_qty: u64,
        max_credit_qty: u64,
        nonce: u64,
        deadline: i64,
    },
    /// Destination-chain remote Relay sell consuming recorded inventory.
    ExecuteRemoteRelaySell {
        relay_order_id: [u8; 32],
        funding_chain_id: u64,
        sell_quantity: u64,
        min_return_usdc: u64,
        nonce: u64,
        deadline: i64,
        relay_ix_data: Vec<u8>,
    },
    /// Funding-chain credit of verified USDC return from a remote sell.
    CreditUsdcReturn {
        relay_order_id: [u8; 32],
        funding_chain_id: u64,
        gross_return_usdc: u64,
        quantity_released: u64,
        cost_released_usdc: u64,
        platform_fee_usdc: u64,
        nonce: u64,
        deadline: i64,
    },
    /// Funding-chain release of a refunded Relay deposit lock.
    ReleaseRelayDeposit {
        relay_order_id: [u8; 32],
        refund_amount_usdc: u64,
        locked_cost_usdc: u64,
        nonce: u64,
        deadline: i64,
    },
    /// Destination-chain restoration of remote inventory after a sell refund.
    RestoreRemoteRelayAsset {
        relay_order_id: [u8; 32],
        funding_chain_id: u64,
        restore_quantity: u64,
        restore_cost_usdc: u64,
        nonce: u64,
        deadline: i64,
    },
    /// Funding-chain Relay gas top-up debiting strategy overhead without deployed lock.
    ExecuteRelayGasTopUp {
        relay_order_id: [u8; 32],
        overhead_usdc: u64,
        gas_recipient: Pubkey,
        nonce: u64,
        deadline: i64,
        relay_ix_data: Vec<u8>,
    },
    /// Funding-chain release of a refunded Relay gas top-up overhead reservation.
    ReleaseRelayGasTopUp {
        relay_order_id: [u8; 32],
        refund_usdc: u64,
        nonce: u64,
        deadline: i64,
    },
}
