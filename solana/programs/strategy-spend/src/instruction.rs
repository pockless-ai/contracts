use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

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
    /// Session executes one pinned Jupiter swap through program-controlled vaults.
    /// `usdc_amount` is the maximum input on buys and minimum output on sells.
    /// `token_amount` is the minimum output on buys and maximum input on sells.
    ExecuteSwap {
        is_buy: bool,
        usdc_amount: u64,
        token_amount: u64,
        jupiter_data: Vec<u8>,
    },
    /// Session executes platform fee, gas reimbursement, and one pinned Jupiter swap atomically.
    /// `gas_jupiter_data` is `[gas_jupiter_account_count: u8][jupiter_ix_data…]`.
    /// Fee and gas amounts reduce `capacity_usdc`, not `deployed_usdc`.
    ExecuteSwapWithFees {
        is_buy: bool,
        usdc_amount: u64,
        token_amount: u64,
        platform_fee_usdc: u64,
        gas_reimburse_usdc: u64,
        treasury: Pubkey,
        jupiter_data: Vec<u8>,
        gas_jupiter_data: Vec<u8>,
    },
    /// Owner withdraws tokens from the strategy vault.
    WithdrawAsset { amount: u64 },
    /// Owner closes the strategy after all positions are flat.
    CloseStrategy,
}
