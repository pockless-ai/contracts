use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

pub const WALLET_SEED: &[u8] = b"wallet";
pub const STRATEGY_SEED: &[u8] = b"strategy";
pub const AUTHORITY_SEED: &[u8] = b"authority";
pub const VAULT_SEED: &[u8] = b"vault";
pub const ASSET_SEED: &[u8] = b"asset";
pub const RELAY_RECEIPT_SEED: &[u8] = b"relay-receipt";
pub const RELAY_PENDING_DEPOSIT_SEED: &[u8] = b"relay-pending-deposit";
pub const RELAY_PENDING_SELL_SEED: &[u8] = b"relay-pending-sell";
pub const REMOTE_ASSET_SEED: &[u8] = b"remote-asset";
pub const REMOTE_AGGREGATE_SEED: &[u8] = b"remote-aggregate";
pub const NATIVE_VAULT_SEED: &[u8] = b"native-vault";

pub const RELAY_ACTION_DEPOSIT: u8 = 0;
pub const RELAY_ACTION_CREDIT_ASSET: u8 = 1;
pub const RELAY_ACTION_REMOTE_SELL: u8 = 2;
pub const RELAY_ACTION_USDC_RETURN: u8 = 3;
pub const RELAY_ACTION_DEPOSIT_RELEASE: u8 = 4;
pub const RELAY_ACTION_ASSET_RESTORE: u8 = 5;
pub const RELAY_ACTION_GAS_TOP_UP: u8 = 6;
pub const RELAY_ACTION_GAS_TOP_UP_RELEASE: u8 = 7;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct WalletConfig {
    pub owner: Pubkey,
    pub usdc_mint: Pubkey,
    pub token_program: Pubkey,
    pub ata_program: Pubkey,
    pub jupiter_program: Pubkey,
    pub relay_depository_program: Pubkey,
    pub platform_relayer: Pubkey,
    pub authority_bump: u8,
}

impl WalletConfig {
    pub const LEN: usize = 32 + 32 + 32 + 32 + 32 + 32 + 32 + 1;
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

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct RemoteStrategyAsset {
    pub quantity: u64,
    pub cost_usdc: u64,
    pub funding_chain_id: u64,
    pub bump: u8,
}

impl RemoteStrategyAsset {
    pub const LEN: usize = 8 + 8 + 8 + 1;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct RemoteMintAggregate {
    pub total_accounted: u64,
    pub bump: u8,
}

impl RemoteMintAggregate {
    pub const LEN: usize = 8 + 1;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct RelayReceipt {
    pub action: u8,
    pub bump: u8,
}

impl RelayReceipt {
    pub const LEN: usize = 1 + 1;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct RelayPendingDeposit {
    pub strategy_id: [u8; 32],
    pub locked_cost_usdc: u64,
    pub origin_amount: u64,
    pub funding_chain_id: u64,
    pub bump: u8,
}

impl RelayPendingDeposit {
    pub const LEN: usize = 32 + 8 + 8 + 8 + 1;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct RelayPendingSell {
    pub strategy_id: [u8; 32],
    pub sell_quantity: u64,
    pub provisional_cost_usdc: u64,
    pub funding_chain_id: u64,
    pub bump: u8,
}

impl RelayPendingSell {
    pub const LEN: usize = 32 + 8 + 8 + 8 + 1;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct RelayPendingGasTopUp {
    pub strategy_id: [u8; 32],
    pub overhead_usdc: u64,
    pub gas_recipient: Pubkey,
    pub bump: u8,
}

impl RelayPendingGasTopUp {
    pub const LEN: usize = 32 + 8 + 32 + 1;
}
