use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

pub const WALLET_CONFIG_VERSION: u8 = 1;

pub const WALLET_SEED: &[u8] = b"wallet";
pub const STRATEGY_SEED: &[u8] = b"strategy";
pub const AUTHORITY_SEED: &[u8] = b"authority";
pub const VAULT_SEED: &[u8] = b"vault";
pub const ASSET_SEED: &[u8] = b"asset";

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct WalletConfig {
    pub version: u8,
    pub owner: Pubkey,
    pub usdc_mint: Pubkey,
    pub token_program: Pubkey,
    pub associated_token_program: Pubkey,
    pub jupiter_program: Pubkey,
    pub authority_bump: u8,
}

impl WalletConfig {
    pub const LEN: usize = 1 + 32 + 32 + 32 + 32 + 32 + 1;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct StrategyAccount {
    pub strategy_id: [u8; 32],
    pub owner: Pubkey,
    pub session: Pubkey,
    pub limit_usdc: u64,
    pub capacity_usdc: u64,
    pub deployed_usdc: u64,
    pub expires_at: i64,
    pub nonce: u64,
    pub revoked: bool,
    pub vault_bump: u8,
}

impl StrategyAccount {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 1 + 1;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct StrategyAsset {
    pub quantity: u64,
    pub cost_usdc: u64,
}

impl StrategyAsset {
    pub const LEN: usize = 8 + 8;
}
