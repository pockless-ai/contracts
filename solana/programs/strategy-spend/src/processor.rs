use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};
use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint},
};

use crate::error::StrategySpendError;
use crate::instruction::{GasMode, StrategySpendInstruction};
use crate::state::{
    RelayPendingDeposit, RelayPendingGasTopUp, RelayPendingSell, RelayReceipt, RemoteMintAggregate,
    RemoteStrategyAsset, StrategyAccount, StrategyAsset, WalletConfig, ASSET_SEED, AUTHORITY_SEED,
    RELAY_ACTION_ASSET_RESTORE, RELAY_ACTION_CREDIT_ASSET, RELAY_ACTION_DEPOSIT,
    RELAY_ACTION_DEPOSIT_RELEASE, RELAY_ACTION_GAS_TOP_UP, RELAY_ACTION_GAS_TOP_UP_RELEASE,
    RELAY_ACTION_REMOTE_SELL, RELAY_ACTION_USDC_RETURN, RELAY_PENDING_DEPOSIT_SEED,
    RELAY_PENDING_SELL_SEED, RELAY_RECEIPT_SEED, REMOTE_AGGREGATE_SEED, REMOTE_ASSET_SEED,
    STRATEGY_SEED, VAULT_SEED, WALLET_SEED,
};

const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    solana_program::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = StrategySpendInstruction::try_from_slice(instruction_data)
        .map_err(|_| StrategySpendError::InvalidInstruction)?;
    match instruction {
        StrategySpendInstruction::InitWallet => init_wallet(program_id, accounts),
        StrategySpendInstruction::InitStrategy {
            strategy_id,
            session,
            limit_usdc,
            expires_at,
        } => init_strategy(
            program_id,
            accounts,
            strategy_id,
            session,
            limit_usdc,
            expires_at,
        ),
        StrategySpendInstruction::SetLimit {
            limit_usdc,
            expires_at,
        } => set_limit(program_id, accounts, limit_usdc, expires_at),
        StrategySpendInstruction::RotateSession { new_session } => {
            rotate_session(program_id, accounts, new_session)
        }
        StrategySpendInstruction::Revoke => revoke(program_id, accounts),
        StrategySpendInstruction::WithdrawAsset { amount } => {
            withdraw_asset(program_id, accounts, amount)
        }
        StrategySpendInstruction::CloseStrategy => close_strategy(program_id, accounts),
        StrategySpendInstruction::ExecuteSwapWithFees {
            is_buy,
            usdc_amount,
            token_amount,
            platform_fee_usdc,
            gas_mode,
            gas_top_up_usdc,
            native_amount,
            treasury,
            gas_recipient,
            jupiter_data,
            gas_jupiter_data,
        } => execute_swap_with_fees(
            program_id,
            accounts,
            is_buy,
            usdc_amount,
            token_amount,
            platform_fee_usdc,
            gas_mode,
            gas_top_up_usdc,
            native_amount,
            treasury,
            gas_recipient,
            jupiter_data,
            gas_jupiter_data,
        ),
        StrategySpendInstruction::ExecuteRelayDeposit {
            relay_order_id,
            funding_chain_id,
            amount,
            min_dest_amount,
            locked_cost_usdc,
            platform_fee_usdc,
            nonce,
            deadline,
            relay_ix_data,
        } => execute_relay_deposit(
            program_id,
            accounts,
            relay_order_id,
            funding_chain_id,
            amount,
            min_dest_amount,
            locked_cost_usdc,
            platform_fee_usdc,
            nonce,
            deadline,
            relay_ix_data,
        ),
        StrategySpendInstruction::CreditRelayAsset {
            relay_order_id,
            funding_chain_id,
            credit_quantity,
            cost_usdc,
            min_credit_qty,
            max_credit_qty,
            nonce,
            deadline,
        } => credit_relay_asset(
            program_id,
            accounts,
            relay_order_id,
            funding_chain_id,
            credit_quantity,
            cost_usdc,
            min_credit_qty,
            max_credit_qty,
            nonce,
            deadline,
        ),
        StrategySpendInstruction::ExecuteRemoteRelaySell {
            relay_order_id,
            funding_chain_id,
            sell_quantity,
            min_return_usdc,
            nonce,
            deadline,
            relay_ix_data,
        } => execute_remote_relay_sell(
            program_id,
            accounts,
            relay_order_id,
            funding_chain_id,
            sell_quantity,
            min_return_usdc,
            nonce,
            deadline,
            relay_ix_data,
        ),
        StrategySpendInstruction::CreditUsdcReturn {
            relay_order_id,
            funding_chain_id,
            gross_return_usdc,
            quantity_released,
            cost_released_usdc,
            platform_fee_usdc,
            nonce,
            deadline,
        } => credit_usdc_return(
            program_id,
            accounts,
            relay_order_id,
            funding_chain_id,
            gross_return_usdc,
            quantity_released,
            cost_released_usdc,
            platform_fee_usdc,
            nonce,
            deadline,
        ),
        StrategySpendInstruction::ReleaseRelayDeposit {
            relay_order_id,
            refund_amount_usdc,
            locked_cost_usdc,
            nonce,
            deadline,
        } => release_relay_deposit(
            program_id,
            accounts,
            relay_order_id,
            refund_amount_usdc,
            locked_cost_usdc,
            nonce,
            deadline,
        ),
        StrategySpendInstruction::RestoreRemoteRelayAsset {
            relay_order_id,
            funding_chain_id,
            restore_quantity,
            restore_cost_usdc,
            nonce,
            deadline,
        } => restore_remote_relay_asset(
            program_id,
            accounts,
            relay_order_id,
            funding_chain_id,
            restore_quantity,
            restore_cost_usdc,
            nonce,
            deadline,
        ),
        StrategySpendInstruction::ExecuteRelayGasTopUp {
            relay_order_id,
            overhead_usdc,
            gas_recipient,
            nonce,
            deadline,
            relay_ix_data,
        } => execute_relay_gas_top_up(
            program_id,
            accounts,
            relay_order_id,
            overhead_usdc,
            gas_recipient,
            nonce,
            deadline,
            relay_ix_data,
        ),
        StrategySpendInstruction::ReleaseRelayGasTopUp {
            relay_order_id,
            refund_usdc,
            nonce,
            deadline,
        } => release_relay_gas_top_up(
            program_id,
            accounts,
            relay_order_id,
            refund_usdc,
            nonce,
            deadline,
        ),
    }
}

fn init_wallet(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let usdc_mint = next_account_info(account_iter)?;
    let jupiter_program = next_account_info(account_iter)?;
    let relay_depository_program = next_account_info(account_iter)?;
    let platform_relayer = next_account_info(account_iter)?;
    let system_program_account = next_account_info(account_iter)?;

    if !owner.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }
    assert_system_program(system_program_account)?;

    let (expected_wallet, wallet_bump) =
        Pubkey::find_program_address(&[WALLET_SEED, owner.key.as_ref()], program_id);
    if wallet.key != &expected_wallet {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let (_, authority_bump) =
        Pubkey::find_program_address(&[AUTHORITY_SEED, owner.key.as_ref()], program_id);

    if wallet.data_is_empty() {
        create_pda(
            owner,
            wallet,
            system_program_account,
            program_id,
            WalletConfig::LEN,
            &[WALLET_SEED, owner.key.as_ref(), &[wallet_bump]],
        )?;
    } else {
        migrate_wallet_account(program_id, owner, wallet, system_program_account)?;
    }

    WalletConfig {
        owner: *owner.key,
        usdc_mint: *usdc_mint.key,
        token_program: spl_token::id(),
        ata_program: ASSOCIATED_TOKEN_PROGRAM_ID,
        jupiter_program: *jupiter_program.key,
        relay_depository_program: *relay_depository_program.key,
        platform_relayer: *platform_relayer.key,
        authority_bump,
    }
    .serialize(&mut &mut wallet.data.borrow_mut()[..])?;
    Ok(())
}

fn init_strategy(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    strategy_id: [u8; 32],
    session: Pubkey,
    limit_usdc: u64,
    expires_at: i64,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let system_program_account = next_account_info(account_iter)?;

    if !owner.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }
    if limit_usdc == 0 || session == Pubkey::default() || expires_at <= Clock::get()?.unix_timestamp
    {
        return Err(StrategySpendError::InvalidInstruction.into());
    }
    assert_system_program(system_program_account)?;
    let _wallet_config = load_wallet(program_id, owner.key, wallet)?;

    let (expected_strategy, strategy_bump) = Pubkey::find_program_address(
        &[STRATEGY_SEED, owner.key.as_ref(), strategy_id.as_ref()],
        program_id,
    );
    if strategy.key != &expected_strategy {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    if !strategy.data_is_empty() {
        return Err(StrategySpendError::AlreadyInitialized.into());
    }

    let (_, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.key.as_ref()], program_id);

    create_pda(
        owner,
        strategy,
        system_program_account,
        program_id,
        StrategyAccount::LEN,
        &[
            STRATEGY_SEED,
            owner.key.as_ref(),
            strategy_id.as_ref(),
            &[strategy_bump],
        ],
    )?;

    StrategyAccount {
        strategy_id,
        owner: *owner.key,
        session,
        limit_usdc,
        capacity_usdc: limit_usdc,
        deployed_usdc: 0,
        expires_at,
        nonce: 0,
        revoked: false,
        vault_bump,
    }
    .serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

fn set_limit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    limit_usdc: u64,
    expires_at: i64,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;

    if !owner.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }
    if limit_usdc == 0 || expires_at <= Clock::get()?.unix_timestamp {
        return Err(StrategySpendError::InvalidInstruction.into());
    }

    let _wallet_config = load_wallet(program_id, owner.key, wallet)?;
    let mut state = load_strategy(program_id, strategy)?;
    if state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }

    let old_limit = state.limit_usdc;
    if limit_usdc > old_limit {
        state.capacity_usdc = state
            .capacity_usdc
            .checked_add(limit_usdc - old_limit)
            .ok_or(StrategySpendError::Overflow)?;
    } else if limit_usdc < old_limit {
        state.capacity_usdc = state.capacity_usdc.min(limit_usdc);
    }
    state.limit_usdc = limit_usdc;
    state.expires_at = expires_at;
    state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

fn rotate_session(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_session: Pubkey,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;

    if !owner.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }
    if new_session == Pubkey::default() {
        return Err(StrategySpendError::InvalidInstruction.into());
    }

    let _wallet_config = load_wallet(program_id, owner.key, wallet)?;
    let mut state = load_strategy(program_id, strategy)?;
    if state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }

    state.session = new_session;
    state.revoked = false;
    state.nonce = state
        .nonce
        .checked_add(1)
        .ok_or(StrategySpendError::Overflow)?;
    state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

fn revoke(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let authority = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;

    if !authority.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }

    let mut state = load_strategy(program_id, strategy)?;
    if authority.key != &state.owner && authority.key != &state.session {
        return Err(StrategySpendError::MissingSignature.into());
    }
    state.revoked = true;
    state.nonce = state
        .nonce
        .checked_add(1)
        .ok_or(StrategySpendError::Overflow)?;
    state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn execute_swap_with_fees(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
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
) -> ProgramResult {
    let relayer = accounts.get(1).ok_or(StrategySpendError::InvalidAccount)?;
    if gas_recipient != *relayer.key {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let (gas_reimburse_usdc, capacity_gas_usdc, min_native_out, native_output) = match gas_mode {
        GasMode::None => {
            if gas_top_up_usdc != 0 || native_amount != 0 || !gas_jupiter_data.is_empty() {
                return Err(StrategySpendError::InvalidInstruction.into());
            }
            (0, 0, 0, None)
        }
        GasMode::CreditOnly => {
            if gas_top_up_usdc != 0 || native_amount != 0 || !gas_jupiter_data.is_empty() {
                return Err(StrategySpendError::InvalidInstruction.into());
            }
            (0, 0, 0, None)
        }
        GasMode::Separate => {
            if gas_top_up_usdc == 0 || native_amount == 0 || gas_jupiter_data.len() <= 1 {
                return Err(StrategySpendError::InvalidInstruction.into());
            }
            (gas_top_up_usdc, 0, native_amount, None)
        }
        GasMode::NativeOutput => {
            if !is_buy || gas_top_up_usdc == 0 || native_amount == 0 || !gas_jupiter_data.is_empty()
            {
                return Err(StrategySpendError::InvalidInstruction.into());
            }
            (0, gas_top_up_usdc, 0, Some(native_amount))
        }
    };

    execute_swap_with_fees_impl(
        program_id,
        accounts,
        is_buy,
        usdc_amount,
        token_amount,
        platform_fee_usdc,
        gas_reimburse_usdc,
        capacity_gas_usdc,
        min_native_out,
        native_output,
        treasury,
        jupiter_data,
        gas_jupiter_data,
    )
}

#[allow(clippy::too_many_arguments)]
fn execute_swap_with_fees_impl(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    is_buy: bool,
    usdc_amount: u64,
    token_amount: u64,
    platform_fee_usdc: u64,
    gas_reimburse_usdc: u64,
    capacity_gas_usdc: u64,
    min_native_out: u64,
    native_output: Option<u64>,
    treasury: Pubkey,
    jupiter_data: Vec<u8>,
    gas_jupiter_data: Vec<u8>,
) -> ProgramResult {
    if jupiter_data.is_empty() || usdc_amount == 0 || token_amount == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }
    validate_fee_fields(
        platform_fee_usdc,
        gas_reimburse_usdc,
        min_native_out,
        treasury,
        &gas_jupiter_data,
    )?;

    let account_iter = &mut accounts.iter();
    let session = next_account_info(account_iter)?;
    let relayer = next_account_info(account_iter)?;
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let vault_authority = next_account_info(account_iter)?;
    let owner_usdc = next_account_info(account_iter)?;
    let treasury_usdc = next_account_info(account_iter)?;
    let strategy_usdc = next_account_info(account_iter)?;
    let strategy_token_vault = next_account_info(account_iter)?;
    let asset_account = next_account_info(account_iter)?;
    let token_mint = next_account_info(account_iter)?;
    let usdc_mint = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;
    let associated_token_program = next_account_info(account_iter)?;
    let system_program_account = next_account_info(account_iter)?;
    let program_authority = next_account_info(account_iter)?;
    let jupiter_program = next_account_info(account_iter)?;
    let gas_wsol = next_account_info(account_iter)?;

    if !session.is_signer || !relayer.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }
    if !relayer.is_writable {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let wallet_config = load_wallet(program_id, owner.key, wallet)?;
    let mut strategy_state = load_strategy(program_id, strategy)?;
    assert_active_strategy(&strategy_state, session.key)?;
    if strategy_state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }

    assert_system_program(system_program_account)?;
    assert_token_program(token_program, &wallet_config)?;
    assert_associated_token_program(associated_token_program, &wallet_config)?;
    if usdc_mint.key != &wallet_config.usdc_mint {
        return Err(StrategySpendError::MintMismatch.into());
    }
    if jupiter_program.key != &wallet_config.jupiter_program {
        return Err(StrategySpendError::ProgramMismatch.into());
    }
    if native_output.is_some() && token_mint.key != &spl_token::native_mint::id() {
        return Err(StrategySpendError::MintMismatch.into());
    }

    let (expected_authority, authority_bump) =
        Pubkey::find_program_address(&[AUTHORITY_SEED, owner.key.as_ref()], program_id);
    if program_authority.key != &expected_authority
        || wallet_config.authority_bump != authority_bump
    {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let (expected_vault_authority, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.key.as_ref()], program_id);
    if vault_authority.key != &expected_vault_authority || strategy_state.vault_bump != vault_bump {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    ensure_vault_ata(
        relayer,
        strategy_usdc,
        vault_authority,
        usdc_mint,
        token_program,
        associated_token_program,
        system_program_account,
    )?;
    ensure_vault_ata(
        relayer,
        strategy_token_vault,
        vault_authority,
        token_mint,
        token_program,
        associated_token_program,
        system_program_account,
    )?;
    assert_strategy_vault(
        strategy_usdc,
        vault_authority.key,
        usdc_mint.key,
        token_program.key,
    )?;
    assert_strategy_vault(
        strategy_token_vault,
        vault_authority.key,
        token_mint.key,
        token_program.key,
    )?;

    let (expected_asset, asset_bump) = Pubkey::find_program_address(
        &[ASSET_SEED, strategy.key.as_ref(), token_mint.key.as_ref()],
        program_id,
    );
    if asset_account.key != &expected_asset {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    ensure_asset_account(
        asset_account,
        strategy,
        token_mint,
        program_id,
        relayer,
        system_program_account,
        asset_bump,
    )?;

    let owner_usdc_expected = associated_token_address(owner.key, &wallet_config.usdc_mint);
    if owner_usdc.key != &owner_usdc_expected {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    assert_usdc_account(owner_usdc, owner.key, &wallet_config.usdc_mint)?;

    if platform_fee_usdc > 0 {
        let treasury_usdc_expected = associated_token_address(&treasury, &wallet_config.usdc_mint);
        if treasury_usdc.key != &treasury_usdc_expected {
            return Err(StrategySpendError::InvalidAccount.into());
        }
        assert_usdc_account(treasury_usdc, &treasury, &wallet_config.usdc_mint)?;
    }

    let protected_accounts = [
        session.key,
        relayer.key,
        owner.key,
        wallet.key,
        strategy.key,
        owner_usdc.key,
        treasury_usdc.key,
        asset_account.key,
        program_authority.key,
    ];

    let fee_capacity_cost = platform_fee_usdc
        .checked_add(gas_reimburse_usdc)
        .and_then(|amount| amount.checked_add(capacity_gas_usdc))
        .ok_or(StrategySpendError::Overflow)?;
    let (gas_account_count, gas_jupiter_ix_data) =
        parse_gas_jupiter_data(&gas_jupiter_data, gas_reimburse_usdc)?;
    let remaining_accounts = account_iter.cloned().collect::<Vec<_>>();
    if remaining_accounts.len() < gas_account_count {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let (gas_accounts, strategy_accounts) = remaining_accounts.split_at(gas_account_count);

    if is_buy {
        let deployable = strategy_state
            .capacity_usdc
            .checked_sub(strategy_state.deployed_usdc)
            .ok_or(StrategySpendError::CapacityExceeded)?;
        let required = usdc_amount
            .checked_add(fee_capacity_cost)
            .ok_or(StrategySpendError::Overflow)?;
        if required > deployable {
            return Err(StrategySpendError::CapacityExceeded.into());
        }
        if fee_capacity_cost > 0 {
            strategy_state.capacity_usdc = strategy_state
                .capacity_usdc
                .checked_sub(fee_capacity_cost)
                .ok_or(StrategySpendError::Overflow)?;
        }
        charge_platform_fee(
            token_program,
            owner_usdc,
            usdc_mint,
            treasury_usdc,
            program_authority,
            owner,
            authority_bump,
            platform_fee_usdc,
        )?;
        reimburse_gas(
            jupiter_program,
            gas_accounts,
            gas_jupiter_ix_data,
            token_program,
            owner_usdc,
            usdc_mint,
            strategy_usdc,
            program_authority,
            owner,
            authority_bump,
            vault_authority,
            strategy,
            vault_bump,
            relayer,
            gas_reimburse_usdc,
            min_native_out,
            gas_wsol,
            &protected_accounts,
        )?;
        if let Some(native_amount) = native_output {
            perform_native_output_buy_swap(
                token_program,
                owner_usdc,
                usdc_mint,
                token_mint,
                strategy_usdc,
                strategy_token_vault,
                gas_wsol,
                asset_account,
                program_authority,
                owner,
                authority_bump,
                jupiter_program,
                strategy_accounts,
                &jupiter_data,
                vault_authority,
                strategy,
                vault_bump,
                &protected_accounts,
                &[session.key, relayer.key],
                relayer,
                &mut strategy_state,
                usdc_amount,
                capacity_gas_usdc,
                token_amount,
                native_amount,
            )?;
        } else {
            perform_buy_swap(
                token_program,
                owner_usdc,
                usdc_mint,
                strategy_usdc,
                strategy_token_vault,
                asset_account,
                program_authority,
                owner,
                authority_bump,
                jupiter_program,
                strategy_accounts,
                &jupiter_data,
                vault_authority,
                strategy,
                vault_bump,
                &protected_accounts,
                &[session.key, relayer.key],
                &mut strategy_state,
                usdc_amount,
                token_amount,
            )?;
        }
    } else {
        perform_sell_swap(
            token_program,
            strategy_usdc,
            usdc_mint,
            strategy_token_vault,
            asset_account,
            owner_usdc,
            vault_authority,
            strategy,
            vault_bump,
            jupiter_program,
            strategy_accounts,
            &jupiter_data,
            &protected_accounts,
            &[session.key, relayer.key],
            &mut strategy_state,
            usdc_amount,
            token_amount,
        )?;
        if fee_capacity_cost > 0 {
            if fee_capacity_cost > strategy_state.capacity_usdc {
                return Err(StrategySpendError::CapacityExceeded.into());
            }
            strategy_state.capacity_usdc = strategy_state
                .capacity_usdc
                .checked_sub(fee_capacity_cost)
                .ok_or(StrategySpendError::Overflow)?;
        }
        charge_platform_fee(
            token_program,
            owner_usdc,
            usdc_mint,
            treasury_usdc,
            program_authority,
            owner,
            authority_bump,
            platform_fee_usdc,
        )?;
        reimburse_gas(
            jupiter_program,
            gas_accounts,
            gas_jupiter_ix_data,
            token_program,
            owner_usdc,
            usdc_mint,
            strategy_usdc,
            program_authority,
            owner,
            authority_bump,
            vault_authority,
            strategy,
            vault_bump,
            relayer,
            gas_reimburse_usdc,
            min_native_out,
            gas_wsol,
            &protected_accounts,
        )?;
    }

    strategy_state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

fn validate_fee_fields(
    platform_fee_usdc: u64,
    gas_reimburse_usdc: u64,
    min_native_out: u64,
    treasury: Pubkey,
    gas_jupiter_data: &[u8],
) -> Result<(), ProgramError> {
    if platform_fee_usdc > 0 && treasury == Pubkey::default() {
        return Err(StrategySpendError::InvalidInstruction.into());
    }
    if gas_reimburse_usdc == 0 {
        if !gas_jupiter_data.is_empty() || min_native_out != 0 {
            return Err(StrategySpendError::InvalidInstruction.into());
        }
    } else if gas_jupiter_data.len() <= 1 || min_native_out == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }
    Ok(())
}

fn parse_gas_jupiter_data(
    gas_jupiter_data: &[u8],
    gas_reimburse_usdc: u64,
) -> Result<(usize, &[u8]), ProgramError> {
    if gas_reimburse_usdc == 0 {
        return Ok((0, &[]));
    }
    let gas_account_count = usize::from(gas_jupiter_data[0]);
    if gas_account_count == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }
    Ok((gas_account_count, &gas_jupiter_data[1..]))
}

#[allow(clippy::too_many_arguments)]
fn charge_platform_fee<'a>(
    token_program: &AccountInfo<'a>,
    owner_usdc: &AccountInfo<'a>,
    usdc_mint: &AccountInfo<'a>,
    treasury_usdc: &AccountInfo<'a>,
    program_authority: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    authority_bump: u8,
    platform_fee_usdc: u64,
) -> ProgramResult {
    if platform_fee_usdc == 0 {
        return Ok(());
    }
    let fee_atomic = scale_to_mint_atomic(platform_fee_usdc, usdc_mint)?;
    invoke_signed(
        &token_instruction::transfer_checked(
            token_program.key,
            owner_usdc.key,
            usdc_mint.key,
            treasury_usdc.key,
            program_authority.key,
            &[],
            fee_atomic,
            mint_decimals(usdc_mint)?,
        )?,
        &[
            owner_usdc.clone(),
            usdc_mint.clone(),
            treasury_usdc.clone(),
            program_authority.clone(),
            token_program.clone(),
        ],
        &[&[AUTHORITY_SEED, owner.key.as_ref(), &[authority_bump]]],
    )
}

#[allow(clippy::too_many_arguments)]
fn reimburse_gas<'a>(
    jupiter_program: &AccountInfo<'a>,
    gas_accounts: &[AccountInfo<'a>],
    gas_jupiter_ix_data: &[u8],
    token_program: &AccountInfo<'a>,
    owner_usdc: &AccountInfo<'a>,
    usdc_mint: &AccountInfo<'a>,
    strategy_usdc: &AccountInfo<'a>,
    program_authority: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    authority_bump: u8,
    vault_authority: &AccountInfo<'a>,
    strategy: &AccountInfo<'a>,
    vault_bump: u8,
    relayer: &AccountInfo<'a>,
    gas_reimburse_usdc: u64,
    min_native_out: u64,
    gas_wsol: &AccountInfo<'a>,
    protected_accounts: &[&Pubkey],
) -> ProgramResult {
    if gas_reimburse_usdc == 0 {
        return Ok(());
    }
    let gas_protected: Vec<&Pubkey> = protected_accounts
        .iter()
        .copied()
        .filter(|key| *key != relayer.key)
        .collect();
    let gas_atomic = scale_to_mint_atomic(gas_reimburse_usdc, usdc_mint)?;
    let strategy_usdc_before = token_account_amount(strategy_usdc)?;
    let expected_gas_wsol =
        associated_token_address(vault_authority.key, &spl_token::native_mint::id());
    if gas_wsol.key != &expected_gas_wsol || !gas_wsol.is_writable {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    assert_native_vault(gas_wsol, vault_authority.key, token_program.key)?;
    if token_account_amount(gas_wsol)? != 0 {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    invoke_signed(
        &token_instruction::transfer_checked(
            token_program.key,
            owner_usdc.key,
            usdc_mint.key,
            strategy_usdc.key,
            program_authority.key,
            &[],
            gas_atomic,
            mint_decimals(usdc_mint)?,
        )?,
        &[
            owner_usdc.clone(),
            usdc_mint.clone(),
            strategy_usdc.clone(),
            program_authority.clone(),
            token_program.clone(),
        ],
        &[&[AUTHORITY_SEED, owner.key.as_ref(), &[authority_bump]]],
    )?;

    cpi_jupiter(
        jupiter_program,
        gas_accounts,
        gas_jupiter_ix_data,
        vault_authority,
        strategy.key,
        vault_bump,
        &gas_protected,
        &[relayer.key],
    )?;

    let strategy_usdc_after = token_account_amount(strategy_usdc)?;
    let funded_balance = strategy_usdc_before
        .checked_add(gas_atomic)
        .ok_or(StrategySpendError::Overflow)?;
    let spent_atomic = funded_balance
        .checked_sub(strategy_usdc_after)
        .ok_or(StrategySpendError::InvalidAccount)?;
    if spent_atomic == 0 || spent_atomic > gas_atomic {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let native_credit = native_token_amount(gas_wsol)?;
    if native_credit < min_native_out {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    invoke_signed(
        &token_instruction::close_account(
            token_program.key,
            gas_wsol.key,
            relayer.key,
            vault_authority.key,
            &[],
        )?,
        &[
            gas_wsol.clone(),
            relayer.clone(),
            vault_authority.clone(),
            token_program.clone(),
        ],
        &[&[VAULT_SEED, strategy.key.as_ref(), &[vault_bump]]],
    )?;
    emit_gas_credit("separate", strategy.key, relayer.key, native_credit);

    let unused = gas_atomic
        .checked_sub(spent_atomic)
        .ok_or(StrategySpendError::InvalidAccount)?;
    if unused > 0 {
        transfer_from_vault(
            token_program,
            strategy_usdc,
            usdc_mint,
            owner_usdc,
            vault_authority,
            strategy,
            vault_bump,
            unused,
        )?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn perform_native_output_buy_swap<'a>(
    token_program: &AccountInfo<'a>,
    owner_usdc: &AccountInfo<'a>,
    usdc_mint: &AccountInfo<'a>,
    token_mint: &AccountInfo<'a>,
    strategy_usdc: &AccountInfo<'a>,
    strategy_token_vault: &AccountInfo<'a>,
    gas_wsol: &AccountInfo<'a>,
    asset_account: &AccountInfo<'a>,
    program_authority: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    authority_bump: u8,
    jupiter_program: &AccountInfo<'a>,
    strategy_accounts: &[AccountInfo<'a>],
    jupiter_data: &[u8],
    vault_authority: &AccountInfo<'a>,
    strategy: &AccountInfo<'a>,
    vault_bump: u8,
    protected_accounts: &[&Pubkey],
    outer_signers: &[&Pubkey],
    relayer: &AccountInfo<'a>,
    strategy_state: &mut StrategyAccount,
    strategy_usdc_amount: u64,
    gas_top_up_usdc: u64,
    minimum_net_token_amount: u64,
    native_amount: u64,
) -> ProgramResult {
    if gas_wsol.key == strategy_token_vault.key || !gas_wsol.is_writable {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    assert_native_vault(gas_wsol, vault_authority.key, token_program.key)?;
    if native_token_amount(gas_wsol)? != 0 {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let total_usdc_amount = strategy_usdc_amount
        .checked_add(gas_top_up_usdc)
        .ok_or(StrategySpendError::Overflow)?;
    let total_usdc_atomic = scale_to_mint_atomic(total_usdc_amount, usdc_mint)?;
    let owner_usdc_before = token_account_amount(owner_usdc)?;
    let strategy_usdc_before = token_account_amount(strategy_usdc)?;
    let token_before = token_account_amount(strategy_token_vault)?;

    invoke_signed(
        &token_instruction::transfer_checked(
            token_program.key,
            owner_usdc.key,
            usdc_mint.key,
            strategy_usdc.key,
            program_authority.key,
            &[],
            total_usdc_atomic,
            mint_decimals(usdc_mint)?,
        )?,
        &[
            owner_usdc.clone(),
            usdc_mint.clone(),
            strategy_usdc.clone(),
            program_authority.clone(),
            token_program.clone(),
        ],
        &[&[AUTHORITY_SEED, owner.key.as_ref(), &[authority_bump]]],
    )?;

    let mut native_output_protected = protected_accounts.to_vec();
    native_output_protected.push(gas_wsol.key);
    cpi_jupiter(
        jupiter_program,
        strategy_accounts,
        jupiter_data,
        vault_authority,
        strategy.key,
        vault_bump,
        &native_output_protected,
        outer_signers,
    )?;

    let strategy_usdc_after = token_account_amount(strategy_usdc)?;
    let funded_balance = strategy_usdc_before
        .checked_add(total_usdc_atomic)
        .ok_or(StrategySpendError::Overflow)?;
    let spent_atomic = funded_balance
        .checked_sub(strategy_usdc_after)
        .ok_or(StrategySpendError::InvalidAccount)?;
    if spent_atomic != total_usdc_atomic {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let owner_usdc_after = token_account_amount(owner_usdc)?;
    if owner_usdc_before
        .checked_sub(owner_usdc_after)
        .ok_or(StrategySpendError::InvalidAccount)?
        != total_usdc_atomic
    {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let token_after = token_account_amount(strategy_token_vault)?;
    let total_received = token_after
        .checked_sub(token_before)
        .ok_or(StrategySpendError::InvalidAccount)?;
    let net_received = total_received
        .checked_sub(native_amount)
        .ok_or(StrategySpendError::InvalidAccount)?;
    if net_received < minimum_net_token_amount {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    transfer_from_vault(
        token_program,
        strategy_token_vault,
        token_mint,
        gas_wsol,
        vault_authority,
        strategy,
        vault_bump,
        native_amount,
    )?;
    let native_credit = native_token_amount(gas_wsol)?;
    if native_credit != native_amount {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    invoke_signed(
        &token_instruction::close_account(
            token_program.key,
            gas_wsol.key,
            relayer.key,
            vault_authority.key,
            &[],
        )?,
        &[
            gas_wsol.clone(),
            relayer.clone(),
            vault_authority.clone(),
            token_program.clone(),
        ],
        &[&[VAULT_SEED, strategy.key.as_ref(), &[vault_bump]]],
    )?;
    emit_gas_credit("native_output", strategy.key, relayer.key, native_credit);

    let mut asset = load_or_default_asset(asset_account)?;
    asset.quantity = asset
        .quantity
        .checked_add(net_received)
        .ok_or(StrategySpendError::Overflow)?;
    asset.cost_usdc = asset
        .cost_usdc
        .checked_add(strategy_usdc_amount)
        .ok_or(StrategySpendError::Overflow)?;
    asset.serialize(&mut &mut asset_account.data.borrow_mut()[..])?;
    strategy_state.deployed_usdc = strategy_state
        .deployed_usdc
        .checked_add(strategy_usdc_amount)
        .ok_or(StrategySpendError::Overflow)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn perform_buy_swap<'a>(
    token_program: &AccountInfo<'a>,
    owner_usdc: &AccountInfo<'a>,
    usdc_mint: &AccountInfo<'a>,
    strategy_usdc: &AccountInfo<'a>,
    strategy_token_vault: &AccountInfo<'a>,
    asset_account: &AccountInfo<'a>,
    program_authority: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    authority_bump: u8,
    jupiter_program: &AccountInfo<'a>,
    strategy_accounts: &[AccountInfo<'a>],
    jupiter_data: &[u8],
    vault_authority: &AccountInfo<'a>,
    strategy: &AccountInfo<'a>,
    vault_bump: u8,
    protected_accounts: &[&Pubkey],
    outer_signers: &[&Pubkey],
    strategy_state: &mut StrategyAccount,
    usdc_amount: u64,
    token_amount: u64,
) -> ProgramResult {
    let deployable = strategy_state
        .capacity_usdc
        .checked_sub(strategy_state.deployed_usdc)
        .ok_or(StrategySpendError::CapacityExceeded)?;
    if usdc_amount > deployable {
        return Err(StrategySpendError::CapacityExceeded.into());
    }

    let max_usdc_atomic = scale_to_mint_atomic(usdc_amount, usdc_mint)?;
    let owner_usdc_before = token_account_amount(owner_usdc)?;
    let strategy_usdc_before = token_account_amount(strategy_usdc)?;
    let token_before = token_account_amount(strategy_token_vault)?;

    invoke_signed(
        &token_instruction::transfer_checked(
            token_program.key,
            owner_usdc.key,
            usdc_mint.key,
            strategy_usdc.key,
            program_authority.key,
            &[],
            max_usdc_atomic,
            mint_decimals(usdc_mint)?,
        )?,
        &[
            owner_usdc.clone(),
            usdc_mint.clone(),
            strategy_usdc.clone(),
            program_authority.clone(),
            token_program.clone(),
        ],
        &[&[AUTHORITY_SEED, owner.key.as_ref(), &[authority_bump]]],
    )?;

    cpi_jupiter(
        jupiter_program,
        strategy_accounts,
        jupiter_data,
        vault_authority,
        strategy.key,
        vault_bump,
        protected_accounts,
        outer_signers,
    )?;

    let strategy_usdc_after_swap = token_account_amount(strategy_usdc)?;
    let funded_balance = strategy_usdc_before
        .checked_add(max_usdc_atomic)
        .ok_or(StrategySpendError::Overflow)?;
    let spent_atomic = funded_balance
        .checked_sub(strategy_usdc_after_swap)
        .ok_or(StrategySpendError::InvalidAccount)?;
    if spent_atomic == 0 || spent_atomic > max_usdc_atomic {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let unused = max_usdc_atomic
        .checked_sub(spent_atomic)
        .ok_or(StrategySpendError::InvalidAccount)?;
    if unused > 0 {
        transfer_from_vault(
            token_program,
            strategy_usdc,
            usdc_mint,
            owner_usdc,
            vault_authority,
            strategy,
            vault_bump,
            unused,
        )?;
    }

    let owner_usdc_after = token_account_amount(owner_usdc)?;
    let owner_spent_atomic = owner_usdc_before
        .checked_sub(owner_usdc_after)
        .ok_or(StrategySpendError::InvalidAccount)?;
    if owner_spent_atomic != spent_atomic {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let spent_usdc = normalize_from_mint_atomic(spent_atomic, usdc_mint)?;
    if spent_usdc == 0 || spent_usdc > usdc_amount {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let token_after = token_account_amount(strategy_token_vault)?;
    let received = token_after
        .checked_sub(token_before)
        .ok_or(StrategySpendError::InvalidAccount)?;
    if received < token_amount {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let mut asset = load_or_default_asset(asset_account)?;
    asset.quantity = asset
        .quantity
        .checked_add(received)
        .ok_or(StrategySpendError::Overflow)?;
    asset.cost_usdc = asset
        .cost_usdc
        .checked_add(spent_usdc)
        .ok_or(StrategySpendError::Overflow)?;
    asset.serialize(&mut &mut asset_account.data.borrow_mut()[..])?;

    strategy_state.deployed_usdc = strategy_state
        .deployed_usdc
        .checked_add(spent_usdc)
        .ok_or(StrategySpendError::Overflow)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn perform_sell_swap<'a>(
    token_program: &AccountInfo<'a>,
    strategy_usdc: &AccountInfo<'a>,
    usdc_mint: &AccountInfo<'a>,
    strategy_token_vault: &AccountInfo<'a>,
    asset_account: &AccountInfo<'a>,
    owner_usdc: &AccountInfo<'a>,
    vault_authority: &AccountInfo<'a>,
    strategy: &AccountInfo<'a>,
    vault_bump: u8,
    jupiter_program: &AccountInfo<'a>,
    strategy_accounts: &[AccountInfo<'a>],
    jupiter_data: &[u8],
    protected_accounts: &[&Pubkey],
    outer_signers: &[&Pubkey],
    strategy_state: &mut StrategyAccount,
    usdc_amount: u64,
    token_amount: u64,
) -> ProgramResult {
    let mut asset = load_or_default_asset(asset_account)?;
    if asset.quantity < token_amount {
        return Err(StrategySpendError::InsufficientAsset.into());
    }

    let token_before = token_account_amount(strategy_token_vault)?;
    let strategy_usdc_before = token_account_amount(strategy_usdc)?;
    cpi_jupiter(
        jupiter_program,
        strategy_accounts,
        jupiter_data,
        vault_authority,
        strategy.key,
        vault_bump,
        protected_accounts,
        outer_signers,
    )?;

    let token_after = token_account_amount(strategy_token_vault)?;
    let token_sold = token_before
        .checked_sub(token_after)
        .ok_or(StrategySpendError::InvalidAccount)?;
    if token_sold == 0 || token_sold > token_amount || asset.quantity < token_sold {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let strategy_usdc_after = token_account_amount(strategy_usdc)?;
    let usdc_received_atomic = strategy_usdc_after
        .checked_sub(strategy_usdc_before)
        .ok_or(StrategySpendError::InvalidAccount)?;
    let usdc_received = normalize_from_mint_atomic(usdc_received_atomic, usdc_mint)?;
    if usdc_received < usdc_amount {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    transfer_from_vault(
        token_program,
        strategy_usdc,
        usdc_mint,
        owner_usdc,
        vault_authority,
        strategy,
        vault_bump,
        usdc_received_atomic,
    )?;

    let cost_sold = pro_rata_cost(asset.cost_usdc, asset.quantity, token_sold)?;
    let realized_pnl = i128::from(usdc_received)
        .checked_sub(i128::from(cost_sold))
        .ok_or(StrategySpendError::Overflow)?;
    let realized_pnl = i64::try_from(realized_pnl).map_err(|_| StrategySpendError::Overflow)?;
    strategy_state.capacity_usdc = apply_realized_pnl(
        strategy_state.capacity_usdc,
        strategy_state.limit_usdc,
        realized_pnl,
    )?;

    asset.quantity = asset
        .quantity
        .checked_sub(token_sold)
        .ok_or(StrategySpendError::Overflow)?;
    asset.cost_usdc = asset
        .cost_usdc
        .checked_sub(cost_sold)
        .ok_or(StrategySpendError::Overflow)?;
    asset.serialize(&mut &mut asset_account.data.borrow_mut()[..])?;

    strategy_state.deployed_usdc = strategy_state
        .deployed_usdc
        .checked_sub(cost_sold)
        .ok_or(StrategySpendError::Overflow)?;
    Ok(())
}

fn funding_chain_seed(funding_chain_id: u64) -> [u8; 8] {
    funding_chain_id.to_le_bytes()
}

fn relay_receipt_pda(
    program_id: &Pubkey,
    strategy: &Pubkey,
    relay_order_id: &[u8; 32],
    action: u8,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            RELAY_RECEIPT_SEED,
            strategy.as_ref(),
            relay_order_id.as_ref(),
            &[action],
        ],
        program_id,
    )
}

fn remote_asset_pda(
    program_id: &Pubkey,
    strategy: &Pubkey,
    mint: &Pubkey,
    funding_chain_id: u64,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            REMOTE_ASSET_SEED,
            strategy.as_ref(),
            mint.as_ref(),
            &funding_chain_seed(funding_chain_id),
        ],
        program_id,
    )
}

fn remote_aggregate_pda(program_id: &Pubkey, strategy: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[REMOTE_AGGREGATE_SEED, strategy.as_ref(), mint.as_ref()],
        program_id,
    )
}

fn relay_pending_deposit_pda(
    program_id: &Pubkey,
    strategy: &Pubkey,
    relay_order_id: &[u8; 32],
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            RELAY_PENDING_DEPOSIT_SEED,
            strategy.as_ref(),
            relay_order_id.as_ref(),
        ],
        program_id,
    )
}

fn relay_pending_sell_pda(
    program_id: &Pubkey,
    strategy: &Pubkey,
    relay_order_id: &[u8; 32],
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            RELAY_PENDING_SELL_SEED,
            strategy.as_ref(),
            relay_order_id.as_ref(),
        ],
        program_id,
    )
}

fn assert_platform_relayer(platform_relayer: &AccountInfo, wallet: &WalletConfig) -> ProgramResult {
    if !platform_relayer.is_signer || platform_relayer.key != &wallet.platform_relayer {
        return Err(StrategySpendError::RelayerMismatch.into());
    }
    Ok(())
}

fn assert_relay_intent(strategy: &StrategyAccount, nonce: u64, deadline: i64) -> ProgramResult {
    if strategy.nonce != nonce {
        return Err(StrategySpendError::NonceMismatch.into());
    }
    if Clock::get()?.unix_timestamp > deadline {
        return Err(StrategySpendError::Expired.into());
    }
    Ok(())
}

fn bump_nonce(strategy_state: &mut StrategyAccount) -> Result<(), ProgramError> {
    strategy_state.nonce = strategy_state
        .nonce
        .checked_add(1)
        .ok_or(StrategySpendError::Overflow)?;
    Ok(())
}

fn consume_relay_receipt<'a>(
    receipt: &AccountInfo<'a>,
    strategy: &AccountInfo<'a>,
    relay_order_id: &[u8; 32],
    action: u8,
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
) -> ProgramResult {
    let (expected, bump) = relay_receipt_pda(program_id, strategy.key, relay_order_id, action);
    if receipt.key != &expected {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    if !receipt.data_is_empty() {
        return Err(StrategySpendError::RelayOrderConsumed.into());
    }
    create_pda(
        payer,
        receipt,
        system_program_account,
        program_id,
        RelayReceipt::LEN,
        &[
            RELAY_RECEIPT_SEED,
            strategy.key.as_ref(),
            relay_order_id.as_ref(),
            &[action],
            &[bump],
        ],
    )?;
    RelayReceipt { action, bump }.serialize(&mut &mut receipt.data.borrow_mut()[..])?;
    Ok(())
}

fn cpi_relay_depository<'account>(
    wallet: &WalletConfig,
    relay_depository: &AccountInfo<'account>,
    relay_ix_data: &[u8],
    remaining: &[AccountInfo<'account>],
    vault_authority: &AccountInfo<'account>,
    strategy: &Pubkey,
    vault_bump: u8,
    protected_accounts: &[&Pubkey],
    outer_signers: &[&Pubkey],
) -> ProgramResult {
    if relay_depository.key != &wallet.relay_depository_program {
        return Err(StrategySpendError::ProgramMismatch.into());
    }
    if relay_ix_data.is_empty() || remaining.is_empty() {
        return Err(StrategySpendError::RelayCpiFailed.into());
    }
    for account in remaining {
        if protected_accounts.contains(&account.key) {
            return Err(StrategySpendError::InvalidAccount.into());
        }
        if account.is_signer
            && account.key != vault_authority.key
            && !outer_signers.contains(&account.key)
        {
            return Err(StrategySpendError::InvalidAccount.into());
        }
    }

    let instruction = solana_program::instruction::Instruction {
        program_id: *relay_depository.key,
        accounts: remaining
            .iter()
            .map(|account| solana_program::instruction::AccountMeta {
                pubkey: *account.key,
                is_signer: account.key == vault_authority.key,
                is_writable: account.is_writable,
            })
            .collect(),
        data: relay_ix_data.to_vec(),
    };
    let mut infos = remaining.to_vec();
    infos.push(relay_depository.clone());
    invoke_signed(
        &instruction,
        &infos,
        &[&[VAULT_SEED, strategy.as_ref(), &[vault_bump]]],
    )
    .map_err(|_| StrategySpendError::RelayCpiFailed)?;
    Ok(())
}

fn create_relay_pending_deposit<'a>(
    pending: &AccountInfo<'a>,
    strategy: &AccountInfo<'a>,
    relay_order_id: &[u8; 32],
    record: RelayPendingDeposit,
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    bump: u8,
) -> ProgramResult {
    if !pending.data_is_empty() {
        return Err(StrategySpendError::AlreadyInitialized.into());
    }
    create_pda(
        payer,
        pending,
        system_program_account,
        program_id,
        RelayPendingDeposit::LEN,
        &[
            RELAY_PENDING_DEPOSIT_SEED,
            strategy.key.as_ref(),
            relay_order_id.as_ref(),
            &[bump],
        ],
    )?;
    record.serialize(&mut &mut pending.data.borrow_mut()[..])?;
    Ok(())
}

fn load_relay_pending_deposit(pending: &AccountInfo) -> Result<RelayPendingDeposit, ProgramError> {
    RelayPendingDeposit::try_from_slice(&pending.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount.into())
}

fn close_relay_pending_deposit<'a>(
    pending: &AccountInfo<'a>,
    recipient: &AccountInfo<'a>,
) -> ProgramResult {
    **recipient.lamports.borrow_mut() = recipient
        .lamports()
        .checked_add(pending.lamports())
        .ok_or(StrategySpendError::Overflow)?;
    **pending.lamports.borrow_mut() = 0;
    pending.assign(&system_program::id());
    pending.realloc(0, false)?;
    Ok(())
}

fn create_relay_pending_sell<'a>(
    pending: &AccountInfo<'a>,
    strategy: &AccountInfo<'a>,
    relay_order_id: &[u8; 32],
    record: RelayPendingSell,
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    bump: u8,
) -> ProgramResult {
    if !pending.data_is_empty() {
        return Err(StrategySpendError::AlreadyInitialized.into());
    }
    create_pda(
        payer,
        pending,
        system_program_account,
        program_id,
        RelayPendingSell::LEN,
        &[
            RELAY_PENDING_SELL_SEED,
            strategy.key.as_ref(),
            relay_order_id.as_ref(),
            &[bump],
        ],
    )?;
    record.serialize(&mut &mut pending.data.borrow_mut()[..])?;
    Ok(())
}

fn load_relay_pending_sell(pending: &AccountInfo) -> Result<RelayPendingSell, ProgramError> {
    RelayPendingSell::try_from_slice(&pending.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount.into())
}

fn close_relay_pending_sell<'a>(
    pending: &AccountInfo<'a>,
    recipient: &AccountInfo<'a>,
) -> ProgramResult {
    close_relay_pending_deposit(pending, recipient)
}

fn require_relay_pending_sell(
    pending_account: &AccountInfo,
    program_id: &Pubkey,
    strategy: &Pubkey,
    strategy_id: &[u8; 32],
    relay_order_id: &[u8; 32],
    funding_chain_id: u64,
    quantity: u64,
    cost_usdc: u64,
) -> Result<RelayPendingSell, ProgramError> {
    let (expected_pending, _) = relay_pending_sell_pda(program_id, strategy, relay_order_id);
    if pending_account.key != &expected_pending {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let pending = load_relay_pending_sell(pending_account)?;
    if pending.strategy_id != *strategy_id
        || pending.sell_quantity != quantity
        || pending.provisional_cost_usdc != cost_usdc
        || pending.funding_chain_id != funding_chain_id
    {
        return Err(StrategySpendError::PendingRecordMissing.into());
    }
    Ok(pending)
}

fn vault_token_surplus(
    vault: &AccountInfo,
    aggregate: &RemoteMintAggregate,
) -> Result<u64, ProgramError> {
    let balance = token_account_amount(vault)?;
    balance
        .checked_sub(aggregate.total_accounted)
        .ok_or(StrategySpendError::InsufficientVaultSurplus.into())
}

fn wrap_vault_native_sol<'a>(
    vault_authority: &AccountInfo<'a>,
    wsol_vault: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    strategy: &Pubkey,
    vault_bump: u8,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Ok(());
    }
    if !vault_authority.is_writable || !wsol_vault.is_writable {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    assert_native_vault(wsol_vault, vault_authority.key, token_program.key)?;
    if vault_authority.lamports() < amount {
        return Err(StrategySpendError::InsufficientVaultSurplus.into());
    }
    invoke_signed(
        &system_instruction::transfer(vault_authority.key, wsol_vault.key, amount),
        &[
            vault_authority.clone(),
            wsol_vault.clone(),
            system_program_account.clone(),
        ],
        &[&[VAULT_SEED, strategy.as_ref(), &[vault_bump]]],
    )?;
    invoke(
        &token_instruction::sync_native(token_program.key, wsol_vault.key)?,
        &[wsol_vault.clone(), token_program.clone()],
    )?;
    Ok(())
}

fn ensure_remote_aggregate<'a>(
    aggregate: &AccountInfo<'a>,
    strategy: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    bump: u8,
) -> ProgramResult {
    if !aggregate.data_is_empty() {
        return Ok(());
    }
    create_pda(
        payer,
        aggregate,
        system_program_account,
        program_id,
        RemoteMintAggregate::LEN,
        &[
            REMOTE_AGGREGATE_SEED,
            strategy.key.as_ref(),
            mint.key.as_ref(),
            &[bump],
        ],
    )?;
    RemoteMintAggregate {
        total_accounted: 0,
        bump,
    }
    .serialize(&mut &mut aggregate.data.borrow_mut()[..])?;
    Ok(())
}

fn ensure_remote_asset<'a>(
    remote_asset: &AccountInfo<'a>,
    strategy: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    funding_chain_id: u64,
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    bump: u8,
) -> ProgramResult {
    if !remote_asset.data_is_empty() {
        return Ok(());
    }
    create_pda(
        payer,
        remote_asset,
        system_program_account,
        program_id,
        RemoteStrategyAsset::LEN,
        &[
            REMOTE_ASSET_SEED,
            strategy.key.as_ref(),
            mint.key.as_ref(),
            &funding_chain_seed(funding_chain_id),
            &[bump],
        ],
    )?;
    RemoteStrategyAsset {
        quantity: 0,
        cost_usdc: 0,
        funding_chain_id,
        bump,
    }
    .serialize(&mut &mut remote_asset.data.borrow_mut()[..])?;
    Ok(())
}

fn load_remote_aggregate(aggregate: &AccountInfo) -> Result<RemoteMintAggregate, ProgramError> {
    RemoteMintAggregate::try_from_slice(&aggregate.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount.into())
}

fn load_remote_asset(remote_asset: &AccountInfo) -> Result<RemoteStrategyAsset, ProgramError> {
    RemoteStrategyAsset::try_from_slice(&remote_asset.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount.into())
}

#[allow(clippy::too_many_arguments)]
fn execute_relay_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    relay_order_id: [u8; 32],
    funding_chain_id: u64,
    amount: u64,
    min_dest_amount: u64,
    locked_cost_usdc: u64,
    platform_fee_usdc: u64,
    nonce: u64,
    deadline: i64,
    relay_ix_data: Vec<u8>,
) -> ProgramResult {
    if amount == 0 || locked_cost_usdc == 0 || locked_cost_usdc > amount || min_dest_amount == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }

    let account_iter = &mut accounts.iter();
    let session = next_account_info(account_iter)?;
    let platform_relayer = next_account_info(account_iter)?;
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let vault_authority = next_account_info(account_iter)?;
    let owner_usdc = next_account_info(account_iter)?;
    let strategy_usdc = next_account_info(account_iter)?;
    let program_authority = next_account_info(account_iter)?;
    let relay_receipt = next_account_info(account_iter)?;
    let relay_pending_deposit = next_account_info(account_iter)?;
    let treasury_usdc = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;
    let associated_token_program = next_account_info(account_iter)?;
    let system_program_account = next_account_info(account_iter)?;
    let relay_depository = next_account_info(account_iter)?;

    if !session.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }

    let wallet_config = load_wallet(program_id, owner.key, wallet)?;
    assert_platform_relayer(platform_relayer, &wallet_config)?;
    let mut strategy_state = load_strategy(program_id, strategy)?;
    assert_active_strategy(&strategy_state, session.key)?;
    if strategy_state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }
    assert_relay_intent(&strategy_state, nonce, deadline)?;

    assert_system_program(system_program_account)?;
    assert_token_program(token_program, &wallet_config)?;
    assert_associated_token_program(associated_token_program, &wallet_config)?;

    let (expected_authority, authority_bump) =
        Pubkey::find_program_address(&[AUTHORITY_SEED, owner.key.as_ref()], program_id);
    if program_authority.key != &expected_authority
        || wallet_config.authority_bump != authority_bump
    {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let (expected_vault_authority, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.key.as_ref()], program_id);
    if vault_authority.key != &expected_vault_authority || strategy_state.vault_bump != vault_bump {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let usdc_mint = next_account_info(account_iter)?;
    if usdc_mint.key != &wallet_config.usdc_mint {
        return Err(StrategySpendError::MintMismatch.into());
    }

    ensure_vault_ata(
        platform_relayer,
        strategy_usdc,
        vault_authority,
        usdc_mint,
        token_program,
        associated_token_program,
        system_program_account,
    )?;
    assert_strategy_vault(
        strategy_usdc,
        vault_authority.key,
        usdc_mint.key,
        token_program.key,
    )?;

    let owner_usdc_expected = associated_token_address(owner.key, &wallet_config.usdc_mint);
    if owner_usdc.key != &owner_usdc_expected {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    assert_usdc_account(owner_usdc, owner.key, &wallet_config.usdc_mint)?;

    let total_cost = locked_cost_usdc
        .checked_add(platform_fee_usdc)
        .ok_or(StrategySpendError::Overflow)?;
    let deployable = strategy_state
        .capacity_usdc
        .checked_sub(strategy_state.deployed_usdc)
        .ok_or(StrategySpendError::CapacityExceeded)?;
    if total_cost > deployable {
        return Err(StrategySpendError::CapacityExceeded.into());
    }

    let deposit_atomic = scale_to_mint_atomic(amount, usdc_mint)?;
    invoke_signed(
        &token_instruction::transfer_checked(
            token_program.key,
            owner_usdc.key,
            usdc_mint.key,
            strategy_usdc.key,
            program_authority.key,
            &[],
            deposit_atomic,
            mint_decimals(usdc_mint)?,
        )?,
        &[
            owner_usdc.clone(),
            usdc_mint.clone(),
            strategy_usdc.clone(),
            program_authority.clone(),
            token_program.clone(),
        ],
        &[&[AUTHORITY_SEED, owner.key.as_ref(), &[authority_bump]]],
    )?;

    if platform_fee_usdc > 0 {
        strategy_state.capacity_usdc = strategy_state
            .capacity_usdc
            .checked_sub(platform_fee_usdc)
            .ok_or(StrategySpendError::Overflow)?;
    }
    strategy_state.deployed_usdc = strategy_state
        .deployed_usdc
        .checked_add(locked_cost_usdc)
        .ok_or(StrategySpendError::Overflow)?;

    let (expected_pending, pending_bump) =
        relay_pending_deposit_pda(program_id, strategy.key, &relay_order_id);
    if relay_pending_deposit.key != &expected_pending {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    create_relay_pending_deposit(
        relay_pending_deposit,
        strategy,
        &relay_order_id,
        RelayPendingDeposit {
            strategy_id: strategy_state.strategy_id,
            locked_cost_usdc,
            origin_amount: amount,
            funding_chain_id,
            bump: pending_bump,
        },
        program_id,
        platform_relayer,
        system_program_account,
        pending_bump,
    )?;

    if platform_fee_usdc > 0 {
        let fee_atomic = scale_to_mint_atomic(platform_fee_usdc, usdc_mint)?;
        transfer_from_vault(
            token_program,
            strategy_usdc,
            usdc_mint,
            treasury_usdc,
            vault_authority,
            strategy,
            vault_bump,
            fee_atomic,
        )?;
    }

    let remaining_accounts = account_iter.cloned().collect::<Vec<_>>();
    let protected = [
        session.key,
        platform_relayer.key,
        owner.key,
        wallet.key,
        strategy.key,
        vault_authority.key,
        owner_usdc.key,
        strategy_usdc.key,
        program_authority.key,
        relay_receipt.key,
        relay_pending_deposit.key,
        treasury_usdc.key,
        usdc_mint.key,
        token_program.key,
        associated_token_program.key,
        system_program_account.key,
        relay_depository.key,
    ];
    cpi_relay_depository(
        &wallet_config,
        relay_depository,
        &relay_ix_data,
        &remaining_accounts,
        vault_authority,
        strategy.key,
        vault_bump,
        &protected,
        &[platform_relayer.key],
    )?;

    consume_relay_receipt(
        relay_receipt,
        strategy,
        &relay_order_id,
        RELAY_ACTION_DEPOSIT,
        program_id,
        platform_relayer,
        system_program_account,
    )?;
    bump_nonce(&mut strategy_state)?;
    strategy_state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    msg!(
        "POCKLESS_RELAY_DEPOSIT:{}:{}:{}",
        strategy.key,
        relay_order_id[0],
        amount
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn credit_relay_asset(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    relay_order_id: [u8; 32],
    funding_chain_id: u64,
    credit_quantity: u64,
    cost_usdc: u64,
    min_credit_qty: u64,
    max_credit_qty: u64,
    nonce: u64,
    deadline: i64,
) -> ProgramResult {
    if credit_quantity == 0 || min_credit_qty == 0 || max_credit_qty < min_credit_qty {
        return Err(StrategySpendError::InvalidInstruction.into());
    }
    if credit_quantity < min_credit_qty || credit_quantity > max_credit_qty {
        return Err(StrategySpendError::RelayCreditBelowMinimum.into());
    }

    let account_iter = &mut accounts.iter();
    let session = next_account_info(account_iter)?;
    let platform_relayer = next_account_info(account_iter)?;
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let vault_authority = next_account_info(account_iter)?;
    let strategy_token_vault = next_account_info(account_iter)?;
    let token_mint = next_account_info(account_iter)?;
    let remote_asset = next_account_info(account_iter)?;
    let remote_aggregate = next_account_info(account_iter)?;
    let relay_receipt = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;
    let system_program_account = next_account_info(account_iter)?;

    if !session.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }

    let wallet_config = load_wallet(program_id, owner.key, wallet)?;
    assert_platform_relayer(platform_relayer, &wallet_config)?;
    let mut strategy_state = load_strategy(program_id, strategy)?;
    assert_active_strategy(&strategy_state, session.key)?;
    if strategy_state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }
    assert_relay_intent(&strategy_state, nonce, deadline)?;

    assert_system_program(system_program_account)?;
    assert_token_program(token_program, &wallet_config)?;

    let (expected_vault_authority, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.key.as_ref()], program_id);
    if vault_authority.key != &expected_vault_authority || strategy_state.vault_bump != vault_bump {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    assert_strategy_vault(
        strategy_token_vault,
        vault_authority.key,
        token_mint.key,
        token_program.key,
    )?;

    let (expected_remote_asset, remote_asset_bump) =
        remote_asset_pda(program_id, strategy.key, token_mint.key, funding_chain_id);
    if remote_asset.key != &expected_remote_asset {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let (expected_aggregate, aggregate_bump) =
        remote_aggregate_pda(program_id, strategy.key, token_mint.key);
    if remote_aggregate.key != &expected_aggregate {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    ensure_remote_aggregate(
        remote_aggregate,
        strategy,
        token_mint,
        program_id,
        platform_relayer,
        system_program_account,
        aggregate_bump,
    )?;
    ensure_remote_asset(
        remote_asset,
        strategy,
        token_mint,
        funding_chain_id,
        program_id,
        platform_relayer,
        system_program_account,
        remote_asset_bump,
    )?;

    let mut aggregate = load_remote_aggregate(remote_aggregate)?;
    let mut surplus = vault_token_surplus(strategy_token_vault, &aggregate)?;
    if surplus < credit_quantity && token_mint.key == &spl_token::native_mint::id() {
        let needed = credit_quantity
            .checked_sub(surplus)
            .ok_or(StrategySpendError::Overflow)?;
        wrap_vault_native_sol(
            vault_authority,
            strategy_token_vault,
            token_program,
            system_program_account,
            strategy.key,
            vault_bump,
            needed,
        )?;
        surplus = vault_token_surplus(strategy_token_vault, &aggregate)?;
    }
    let allowed = surplus.min(max_credit_qty);
    if allowed < min_credit_qty || allowed != credit_quantity {
        return Err(StrategySpendError::InsufficientVaultSurplus.into());
    }

    let mut remote = load_remote_asset(remote_asset)?;
    if remote.funding_chain_id != funding_chain_id {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    remote.quantity = remote
        .quantity
        .checked_add(credit_quantity)
        .ok_or(StrategySpendError::Overflow)?;
    remote.cost_usdc = remote
        .cost_usdc
        .checked_add(cost_usdc)
        .ok_or(StrategySpendError::Overflow)?;
    remote.serialize(&mut &mut remote_asset.data.borrow_mut()[..])?;

    aggregate.total_accounted = aggregate
        .total_accounted
        .checked_add(credit_quantity)
        .ok_or(StrategySpendError::Overflow)?;
    aggregate.serialize(&mut &mut remote_aggregate.data.borrow_mut()[..])?;

    consume_relay_receipt(
        relay_receipt,
        strategy,
        &relay_order_id,
        RELAY_ACTION_CREDIT_ASSET,
        program_id,
        platform_relayer,
        system_program_account,
    )?;
    bump_nonce(&mut strategy_state)?;
    strategy_state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn execute_remote_relay_sell(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    relay_order_id: [u8; 32],
    funding_chain_id: u64,
    sell_quantity: u64,
    min_return_usdc: u64,
    nonce: u64,
    deadline: i64,
    relay_ix_data: Vec<u8>,
) -> ProgramResult {
    if sell_quantity == 0 || min_return_usdc == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }

    let account_iter = &mut accounts.iter();
    let session = next_account_info(account_iter)?;
    let platform_relayer = next_account_info(account_iter)?;
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let vault_authority = next_account_info(account_iter)?;
    let strategy_token_vault = next_account_info(account_iter)?;
    let token_mint = next_account_info(account_iter)?;
    let remote_asset = next_account_info(account_iter)?;
    let remote_aggregate = next_account_info(account_iter)?;
    let relay_receipt = next_account_info(account_iter)?;
    let relay_pending_sell = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;
    let system_program_account = next_account_info(account_iter)?;
    let relay_depository = next_account_info(account_iter)?;

    if !session.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }

    let wallet_config = load_wallet(program_id, owner.key, wallet)?;
    assert_platform_relayer(platform_relayer, &wallet_config)?;
    let mut strategy_state = load_strategy(program_id, strategy)?;
    assert_active_strategy(&strategy_state, session.key)?;
    if strategy_state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }
    assert_relay_intent(&strategy_state, nonce, deadline)?;
    assert_system_program(system_program_account)?;
    assert_token_program(token_program, &wallet_config)?;

    let (expected_vault_authority, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.key.as_ref()], program_id);
    if vault_authority.key != &expected_vault_authority || strategy_state.vault_bump != vault_bump {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    assert_strategy_vault(
        strategy_token_vault,
        vault_authority.key,
        token_mint.key,
        token_program.key,
    )?;

    let (expected_remote_asset, _) =
        remote_asset_pda(program_id, strategy.key, token_mint.key, funding_chain_id);
    if remote_asset.key != &expected_remote_asset {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let (expected_aggregate, _) = remote_aggregate_pda(program_id, strategy.key, token_mint.key);
    if remote_aggregate.key != &expected_aggregate {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let mut remote = load_remote_asset(remote_asset)?;
    if remote.funding_chain_id != funding_chain_id || remote.quantity < sell_quantity {
        return Err(StrategySpendError::InsufficientRemoteAsset.into());
    }

    let cost_released = pro_rata_cost(remote.cost_usdc, remote.quantity, sell_quantity)?;
    remote.quantity = remote
        .quantity
        .checked_sub(sell_quantity)
        .ok_or(StrategySpendError::Overflow)?;
    remote.cost_usdc = remote
        .cost_usdc
        .checked_sub(cost_released)
        .ok_or(StrategySpendError::Overflow)?;
    remote.serialize(&mut &mut remote_asset.data.borrow_mut()[..])?;

    let mut aggregate = load_remote_aggregate(remote_aggregate)?;
    aggregate.total_accounted = aggregate
        .total_accounted
        .checked_sub(sell_quantity)
        .ok_or(StrategySpendError::Overflow)?;
    aggregate.serialize(&mut &mut remote_aggregate.data.borrow_mut()[..])?;

    let (expected_pending, pending_bump) =
        relay_pending_sell_pda(program_id, strategy.key, &relay_order_id);
    if relay_pending_sell.key != &expected_pending {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    create_relay_pending_sell(
        relay_pending_sell,
        strategy,
        &relay_order_id,
        RelayPendingSell {
            strategy_id: strategy_state.strategy_id,
            sell_quantity,
            provisional_cost_usdc: cost_released,
            funding_chain_id,
            bump: pending_bump,
        },
        program_id,
        platform_relayer,
        system_program_account,
        pending_bump,
    )?;

    let remaining_accounts = account_iter.cloned().collect::<Vec<_>>();
    let protected = [
        session.key,
        platform_relayer.key,
        owner.key,
        wallet.key,
        strategy.key,
        vault_authority.key,
        strategy_token_vault.key,
        token_mint.key,
        remote_asset.key,
        remote_aggregate.key,
        relay_receipt.key,
        relay_pending_sell.key,
        token_program.key,
        system_program_account.key,
        relay_depository.key,
    ];
    cpi_relay_depository(
        &wallet_config,
        relay_depository,
        &relay_ix_data,
        &remaining_accounts,
        vault_authority,
        strategy.key,
        vault_bump,
        &protected,
        &[platform_relayer.key],
    )?;

    consume_relay_receipt(
        relay_receipt,
        strategy,
        &relay_order_id,
        RELAY_ACTION_REMOTE_SELL,
        program_id,
        platform_relayer,
        system_program_account,
    )?;
    msg!(
        "POCKLESS_REMOTE_SELL:{}:{}:{}:{}",
        strategy.key,
        relay_order_id[0],
        sell_quantity,
        cost_released
    );
    bump_nonce(&mut strategy_state)?;
    strategy_state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn credit_usdc_return(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    relay_order_id: [u8; 32],
    funding_chain_id: u64,
    gross_return_usdc: u64,
    quantity_released: u64,
    cost_released_usdc: u64,
    platform_fee_usdc: u64,
    nonce: u64,
    deadline: i64,
) -> ProgramResult {
    if gross_return_usdc == 0 || cost_released_usdc == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }
    if platform_fee_usdc > gross_return_usdc {
        return Err(StrategySpendError::InvalidInstruction.into());
    }

    let account_iter = &mut accounts.iter();
    let session = next_account_info(account_iter)?;
    let platform_relayer = next_account_info(account_iter)?;
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let vault_authority = next_account_info(account_iter)?;
    let owner_usdc = next_account_info(account_iter)?;
    let strategy_usdc = next_account_info(account_iter)?;
    let treasury_usdc = next_account_info(account_iter)?;
    let relay_receipt = next_account_info(account_iter)?;
    let relay_pending_sell = next_account_info(account_iter)?;
    let usdc_mint = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;
    let system_program_account = next_account_info(account_iter)?;

    if !session.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }

    let wallet_config = load_wallet(program_id, owner.key, wallet)?;
    assert_platform_relayer(platform_relayer, &wallet_config)?;
    let mut strategy_state = load_strategy(program_id, strategy)?;
    assert_active_strategy(&strategy_state, session.key)?;
    if strategy_state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }
    assert_relay_intent(&strategy_state, nonce, deadline)?;

    assert_system_program(system_program_account)?;
    assert_token_program(token_program, &wallet_config)?;
    if usdc_mint.key != &wallet_config.usdc_mint {
        return Err(StrategySpendError::MintMismatch.into());
    }

    let (expected_vault_authority, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.key.as_ref()], program_id);
    if vault_authority.key != &expected_vault_authority || strategy_state.vault_bump != vault_bump {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    assert_strategy_vault(
        strategy_usdc,
        vault_authority.key,
        usdc_mint.key,
        token_program.key,
    )?;

    let owner_usdc_expected = associated_token_address(owner.key, &wallet_config.usdc_mint);
    if owner_usdc.key != &owner_usdc_expected {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    assert_usdc_account(owner_usdc, owner.key, &wallet_config.usdc_mint)?;

    if cost_released_usdc > strategy_state.deployed_usdc {
        return Err(StrategySpendError::CapacityExceeded.into());
    }

    let net_return = gross_return_usdc
        .checked_sub(platform_fee_usdc)
        .ok_or(StrategySpendError::Overflow)?;
    let return_atomic = scale_to_mint_atomic(net_return, usdc_mint)?;
    transfer_from_vault(
        token_program,
        strategy_usdc,
        usdc_mint,
        owner_usdc,
        vault_authority,
        strategy,
        vault_bump,
        return_atomic,
    )?;

    if platform_fee_usdc > 0 {
        let fee_atomic = scale_to_mint_atomic(platform_fee_usdc, usdc_mint)?;
        transfer_from_vault(
            token_program,
            strategy_usdc,
            usdc_mint,
            treasury_usdc,
            vault_authority,
            strategy,
            vault_bump,
            fee_atomic,
        )?;
        strategy_state.capacity_usdc = strategy_state
            .capacity_usdc
            .checked_sub(platform_fee_usdc)
            .ok_or(StrategySpendError::Overflow)?;
    }

    let realized_pnl = i128::from(gross_return_usdc)
        .checked_sub(i128::from(cost_released_usdc))
        .ok_or(StrategySpendError::Overflow)?;
    let realized_pnl = i64::try_from(realized_pnl).map_err(|_| StrategySpendError::Overflow)?;
    strategy_state.capacity_usdc = apply_realized_pnl(
        strategy_state.capacity_usdc,
        strategy_state.limit_usdc,
        realized_pnl,
    )?;
    strategy_state.deployed_usdc = strategy_state
        .deployed_usdc
        .checked_sub(cost_released_usdc)
        .ok_or(StrategySpendError::Overflow)?;

    require_relay_pending_sell(
        relay_pending_sell,
        program_id,
        strategy.key,
        &strategy_state.strategy_id,
        &relay_order_id,
        funding_chain_id,
        quantity_released,
        cost_released_usdc,
    )?;
    consume_relay_receipt(
        relay_receipt,
        strategy,
        &relay_order_id,
        RELAY_ACTION_USDC_RETURN,
        program_id,
        platform_relayer,
        system_program_account,
    )?;
    close_relay_pending_sell(relay_pending_sell, platform_relayer)?;
    bump_nonce(&mut strategy_state)?;
    strategy_state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn release_relay_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    relay_order_id: [u8; 32],
    refund_amount_usdc: u64,
    locked_cost_usdc: u64,
    nonce: u64,
    deadline: i64,
) -> ProgramResult {
    if refund_amount_usdc == 0 || locked_cost_usdc == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }

    let account_iter = &mut accounts.iter();
    let session = next_account_info(account_iter)?;
    let platform_relayer = next_account_info(account_iter)?;
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let vault_authority = next_account_info(account_iter)?;
    let owner_usdc = next_account_info(account_iter)?;
    let strategy_usdc = next_account_info(account_iter)?;
    let relay_receipt = next_account_info(account_iter)?;
    let relay_pending_deposit = next_account_info(account_iter)?;
    let usdc_mint = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;
    let system_program_account = next_account_info(account_iter)?;

    if !session.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }

    let wallet_config = load_wallet(program_id, owner.key, wallet)?;
    assert_platform_relayer(platform_relayer, &wallet_config)?;
    let mut strategy_state = load_strategy(program_id, strategy)?;
    assert_active_strategy(&strategy_state, session.key)?;
    if strategy_state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }
    assert_relay_intent(&strategy_state, nonce, deadline)?;

    assert_system_program(system_program_account)?;
    assert_token_program(token_program, &wallet_config)?;
    if usdc_mint.key != &wallet_config.usdc_mint {
        return Err(StrategySpendError::MintMismatch.into());
    }

    let (expected_pending, _) =
        relay_pending_deposit_pda(program_id, strategy.key, &relay_order_id);
    if relay_pending_deposit.key != &expected_pending {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let pending = load_relay_pending_deposit(relay_pending_deposit)?;
    if pending.strategy_id != strategy_state.strategy_id
        || pending.locked_cost_usdc != locked_cost_usdc
        || pending.origin_amount != refund_amount_usdc
    {
        return Err(StrategySpendError::PendingRecordMissing.into());
    }

    let (expected_vault_authority, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.key.as_ref()], program_id);
    if vault_authority.key != &expected_vault_authority || strategy_state.vault_bump != vault_bump {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    assert_strategy_vault(
        strategy_usdc,
        vault_authority.key,
        usdc_mint.key,
        token_program.key,
    )?;

    let owner_usdc_expected = associated_token_address(owner.key, &wallet_config.usdc_mint);
    if owner_usdc.key != &owner_usdc_expected {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    assert_usdc_account(owner_usdc, owner.key, &wallet_config.usdc_mint)?;

    if locked_cost_usdc > strategy_state.deployed_usdc {
        return Err(StrategySpendError::CapacityExceeded.into());
    }

    let refund_atomic = scale_to_mint_atomic(refund_amount_usdc, usdc_mint)?;
    transfer_from_vault(
        token_program,
        strategy_usdc,
        usdc_mint,
        owner_usdc,
        vault_authority,
        strategy,
        vault_bump,
        refund_atomic,
    )?;

    strategy_state.deployed_usdc = strategy_state
        .deployed_usdc
        .checked_sub(locked_cost_usdc)
        .ok_or(StrategySpendError::Overflow)?;

    consume_relay_receipt(
        relay_receipt,
        strategy,
        &relay_order_id,
        RELAY_ACTION_DEPOSIT_RELEASE,
        program_id,
        platform_relayer,
        system_program_account,
    )?;
    close_relay_pending_deposit(relay_pending_deposit, platform_relayer)?;
    bump_nonce(&mut strategy_state)?;
    strategy_state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn execute_relay_gas_top_up(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    relay_order_id: [u8; 32],
    overhead_usdc: u64,
    gas_recipient: Pubkey,
    nonce: u64,
    deadline: i64,
    relay_ix_data: Vec<u8>,
) -> ProgramResult {
    if overhead_usdc == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }

    let account_iter = &mut accounts.iter();
    let session = next_account_info(account_iter)?;
    let platform_relayer = next_account_info(account_iter)?;
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let vault_authority = next_account_info(account_iter)?;
    let owner_usdc = next_account_info(account_iter)?;
    let strategy_usdc = next_account_info(account_iter)?;
    let program_authority = next_account_info(account_iter)?;
    let relay_receipt = next_account_info(account_iter)?;
    let relay_pending_gas = next_account_info(account_iter)?;
    let usdc_mint = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;
    let associated_token_program = next_account_info(account_iter)?;
    let system_program_account = next_account_info(account_iter)?;
    let relay_depository = next_account_info(account_iter)?;

    if !session.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }

    let wallet_config = load_wallet(program_id, owner.key, wallet)?;
    assert_platform_relayer(platform_relayer, &wallet_config)?;
    let mut strategy_state = load_strategy(program_id, strategy)?;
    assert_active_strategy(&strategy_state, session.key)?;
    if strategy_state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }
    assert_relay_intent(&strategy_state, nonce, deadline)?;

    assert_system_program(system_program_account)?;
    assert_token_program(token_program, &wallet_config)?;
    assert_associated_token_program(associated_token_program, &wallet_config)?;

    let (expected_authority, authority_bump) =
        Pubkey::find_program_address(&[AUTHORITY_SEED, owner.key.as_ref()], program_id);
    if program_authority.key != &expected_authority
        || wallet_config.authority_bump != authority_bump
    {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let (expected_vault_authority, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.key.as_ref()], program_id);
    if vault_authority.key != &expected_vault_authority || strategy_state.vault_bump != vault_bump {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    if usdc_mint.key != &wallet_config.usdc_mint {
        return Err(StrategySpendError::MintMismatch.into());
    }

    ensure_vault_ata(
        platform_relayer,
        strategy_usdc,
        vault_authority,
        usdc_mint,
        token_program,
        associated_token_program,
        system_program_account,
    )?;

    let owner_usdc_expected = associated_token_address(owner.key, &wallet_config.usdc_mint);
    if owner_usdc.key != &owner_usdc_expected {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let deployable = strategy_state
        .capacity_usdc
        .checked_sub(strategy_state.deployed_usdc)
        .ok_or(StrategySpendError::CapacityExceeded)?;
    if overhead_usdc > deployable {
        return Err(StrategySpendError::CapacityExceeded.into());
    }

    let overhead_atomic = scale_to_mint_atomic(overhead_usdc, usdc_mint)?;
    invoke_signed(
        &token_instruction::transfer_checked(
            token_program.key,
            owner_usdc.key,
            usdc_mint.key,
            strategy_usdc.key,
            program_authority.key,
            &[],
            overhead_atomic,
            mint_decimals(usdc_mint)?,
        )?,
        &[
            owner_usdc.clone(),
            usdc_mint.clone(),
            strategy_usdc.clone(),
            program_authority.clone(),
            token_program.clone(),
        ],
        &[&[AUTHORITY_SEED, owner.key.as_ref(), &[authority_bump]]],
    )?;

    strategy_state.capacity_usdc = strategy_state
        .capacity_usdc
        .checked_sub(overhead_usdc)
        .ok_or(StrategySpendError::Overflow)?;

    let (expected_pending, pending_bump) = Pubkey::find_program_address(
        &[
            b"relay-pending-gas",
            strategy.key.as_ref(),
            relay_order_id.as_ref(),
        ],
        program_id,
    );
    if relay_pending_gas.key != &expected_pending {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    create_pda(
        platform_relayer,
        relay_pending_gas,
        system_program_account,
        program_id,
        RelayPendingGasTopUp::LEN,
        &[
            b"relay-pending-gas",
            strategy.key.as_ref(),
            relay_order_id.as_ref(),
            &[pending_bump],
        ],
    )?;
    RelayPendingGasTopUp {
        strategy_id: strategy_state.strategy_id,
        overhead_usdc,
        gas_recipient,
        bump: pending_bump,
    }
    .serialize(&mut &mut relay_pending_gas.data.borrow_mut()[..])?;

    let remaining_accounts = account_iter.cloned().collect::<Vec<_>>();
    cpi_relay_depository(
        &wallet_config,
        relay_depository,
        &relay_ix_data,
        &remaining_accounts,
        vault_authority,
        strategy.key,
        vault_bump,
        &[],
        &[platform_relayer.key],
    )?;

    consume_relay_receipt(
        relay_receipt,
        strategy,
        &relay_order_id,
        RELAY_ACTION_GAS_TOP_UP,
        program_id,
        platform_relayer,
        system_program_account,
    )?;
    bump_nonce(&mut strategy_state)?;
    strategy_state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

fn release_relay_gas_top_up(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    relay_order_id: [u8; 32],
    refund_usdc: u64,
    nonce: u64,
    deadline: i64,
) -> ProgramResult {
    if refund_usdc == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }

    let account_iter = &mut accounts.iter();
    let session = next_account_info(account_iter)?;
    let platform_relayer = next_account_info(account_iter)?;
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let relay_receipt = next_account_info(account_iter)?;
    let relay_pending_gas = next_account_info(account_iter)?;
    let system_program_account = next_account_info(account_iter)?;

    if !session.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }

    let wallet_config = load_wallet(program_id, owner.key, wallet)?;
    assert_platform_relayer(platform_relayer, &wallet_config)?;
    let mut strategy_state = load_strategy(program_id, strategy)?;
    assert_active_strategy(&strategy_state, session.key)?;
    if strategy_state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }
    assert_relay_intent(&strategy_state, nonce, deadline)?;
    assert_system_program(system_program_account)?;

    let (expected_pending, _) = Pubkey::find_program_address(
        &[
            b"relay-pending-gas",
            strategy.key.as_ref(),
            relay_order_id.as_ref(),
        ],
        program_id,
    );
    if relay_pending_gas.key != &expected_pending {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let pending = RelayPendingGasTopUp::try_from_slice(&relay_pending_gas.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount)?;
    if pending.strategy_id != strategy_state.strategy_id || pending.overhead_usdc != refund_usdc {
        return Err(StrategySpendError::PendingRecordMissing.into());
    }

    strategy_state.capacity_usdc = strategy_state
        .capacity_usdc
        .checked_add(refund_usdc)
        .ok_or(StrategySpendError::Overflow)?;

    consume_relay_receipt(
        relay_receipt,
        strategy,
        &relay_order_id,
        RELAY_ACTION_GAS_TOP_UP_RELEASE,
        program_id,
        platform_relayer,
        system_program_account,
    )?;
    close_relay_pending_deposit(relay_pending_gas, platform_relayer)?;
    bump_nonce(&mut strategy_state)?;
    strategy_state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn restore_remote_relay_asset(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    relay_order_id: [u8; 32],
    funding_chain_id: u64,
    restore_quantity: u64,
    restore_cost_usdc: u64,
    nonce: u64,
    deadline: i64,
) -> ProgramResult {
    if restore_quantity == 0 || restore_cost_usdc == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }

    let account_iter = &mut accounts.iter();
    let session = next_account_info(account_iter)?;
    let platform_relayer = next_account_info(account_iter)?;
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let vault_authority = next_account_info(account_iter)?;
    let strategy_token_vault = next_account_info(account_iter)?;
    let token_mint = next_account_info(account_iter)?;
    let remote_asset = next_account_info(account_iter)?;
    let remote_aggregate = next_account_info(account_iter)?;
    let relay_receipt = next_account_info(account_iter)?;
    let relay_pending_sell = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;
    let system_program_account = next_account_info(account_iter)?;

    if !session.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }

    let wallet_config = load_wallet(program_id, owner.key, wallet)?;
    assert_platform_relayer(platform_relayer, &wallet_config)?;
    let mut strategy_state = load_strategy(program_id, strategy)?;
    assert_active_strategy(&strategy_state, session.key)?;
    if strategy_state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }
    assert_relay_intent(&strategy_state, nonce, deadline)?;
    assert_system_program(system_program_account)?;
    assert_token_program(token_program, &wallet_config)?;

    let (expected_vault_authority, _) =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.key.as_ref()], program_id);
    if vault_authority.key != &expected_vault_authority {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    assert_strategy_vault(
        strategy_token_vault,
        vault_authority.key,
        token_mint.key,
        token_program.key,
    )?;

    let (expected_remote_asset, remote_asset_bump) =
        remote_asset_pda(program_id, strategy.key, token_mint.key, funding_chain_id);
    if remote_asset.key != &expected_remote_asset {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let (expected_aggregate, aggregate_bump) =
        remote_aggregate_pda(program_id, strategy.key, token_mint.key);
    if remote_aggregate.key != &expected_aggregate {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    ensure_remote_aggregate(
        remote_aggregate,
        strategy,
        token_mint,
        program_id,
        platform_relayer,
        system_program_account,
        aggregate_bump,
    )?;
    ensure_remote_asset(
        remote_asset,
        strategy,
        token_mint,
        funding_chain_id,
        program_id,
        platform_relayer,
        system_program_account,
        remote_asset_bump,
    )?;

    let mut aggregate = load_remote_aggregate(remote_aggregate)?;
    let surplus = vault_token_surplus(strategy_token_vault, &aggregate)?;
    if surplus < restore_quantity {
        return Err(StrategySpendError::InsufficientVaultSurplus.into());
    }

    let mut remote = load_remote_asset(remote_asset)?;
    if remote.funding_chain_id != funding_chain_id {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    remote.quantity = remote
        .quantity
        .checked_add(restore_quantity)
        .ok_or(StrategySpendError::Overflow)?;
    remote.cost_usdc = remote
        .cost_usdc
        .checked_add(restore_cost_usdc)
        .ok_or(StrategySpendError::Overflow)?;
    remote.serialize(&mut &mut remote_asset.data.borrow_mut()[..])?;

    aggregate.total_accounted = aggregate
        .total_accounted
        .checked_add(restore_quantity)
        .ok_or(StrategySpendError::Overflow)?;
    aggregate.serialize(&mut &mut remote_aggregate.data.borrow_mut()[..])?;

    require_relay_pending_sell(
        relay_pending_sell,
        program_id,
        strategy.key,
        &strategy_state.strategy_id,
        &relay_order_id,
        funding_chain_id,
        restore_quantity,
        restore_cost_usdc,
    )?;
    consume_relay_receipt(
        relay_receipt,
        strategy,
        &relay_order_id,
        RELAY_ACTION_ASSET_RESTORE,
        program_id,
        platform_relayer,
        system_program_account,
    )?;
    close_relay_pending_sell(relay_pending_sell, platform_relayer)?;
    bump_nonce(&mut strategy_state)?;
    strategy_state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

fn withdraw_asset(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    if amount == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }

    let account_iter = &mut accounts.iter();
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let vault_authority = next_account_info(account_iter)?;
    let strategy_vault = next_account_info(account_iter)?;
    let owner_token = next_account_info(account_iter)?;
    let token_mint = next_account_info(account_iter)?;
    let asset_account = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;
    let associated_token_program = next_account_info(account_iter)?;
    let system_program_account = next_account_info(account_iter)?;

    if !owner.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }

    let wallet_config = load_wallet(program_id, owner.key, wallet)?;
    let mut strategy_state = load_strategy(program_id, strategy)?;
    if strategy_state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }

    assert_token_program(token_program, &wallet_config)?;
    assert_associated_token_program(associated_token_program, &wallet_config)?;
    assert_system_program(system_program_account)?;

    let (expected_vault_authority, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.key.as_ref()], program_id);
    if vault_authority.key != &expected_vault_authority || strategy_state.vault_bump != vault_bump {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    assert_strategy_vault(
        strategy_vault,
        vault_authority.key,
        token_mint.key,
        token_program.key,
    )?;

    let owner_token_expected = associated_token_address(owner.key, token_mint.key);
    if owner_token.key != &owner_token_expected {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let (expected_asset, _) = Pubkey::find_program_address(
        &[ASSET_SEED, strategy.key.as_ref(), token_mint.key.as_ref()],
        program_id,
    );
    if asset_account.key != &expected_asset {
        return Err(StrategySpendError::InvalidAccount.into());
    }

    let mut asset = load_or_default_asset(asset_account)?;
    if asset.quantity < amount {
        return Err(StrategySpendError::InsufficientAsset.into());
    }

    let cost_removed = pro_rata_cost(asset.cost_usdc, asset.quantity, amount)?;

    invoke_signed(
        &token_instruction::transfer_checked(
            token_program.key,
            strategy_vault.key,
            token_mint.key,
            owner_token.key,
            vault_authority.key,
            &[],
            amount,
            mint_decimals(token_mint)?,
        )?,
        &[
            strategy_vault.clone(),
            token_mint.clone(),
            owner_token.clone(),
            vault_authority.clone(),
            token_program.clone(),
        ],
        &[&[VAULT_SEED, strategy.key.as_ref(), &[vault_bump]]],
    )?;

    asset.quantity = asset
        .quantity
        .checked_sub(amount)
        .ok_or(StrategySpendError::Overflow)?;
    asset.cost_usdc = asset
        .cost_usdc
        .checked_sub(cost_removed)
        .ok_or(StrategySpendError::Overflow)?;
    asset.serialize(&mut &mut asset_account.data.borrow_mut()[..])?;

    strategy_state.deployed_usdc = strategy_state
        .deployed_usdc
        .checked_sub(cost_removed)
        .ok_or(StrategySpendError::Overflow)?;
    strategy_state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

fn close_strategy(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let recipient = next_account_info(account_iter)?;

    if !owner.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
    }

    let _wallet_config = load_wallet(program_id, owner.key, wallet)?;
    let strategy_state = load_strategy(program_id, strategy)?;
    if strategy_state.owner != *owner.key {
        return Err(StrategySpendError::OwnerMismatch.into());
    }
    if strategy_state.deployed_usdc != 0 {
        return Err(StrategySpendError::OpenDeployment.into());
    }

    let dest = if recipient.key == owner.key {
        owner
    } else {
        recipient
    };

    **dest.lamports.borrow_mut() = dest
        .lamports()
        .checked_add(strategy.lamports())
        .ok_or(StrategySpendError::Overflow)?;
    **strategy.lamports.borrow_mut() = 0;
    strategy.assign(&system_program::id());
    strategy.realloc(0, false)?;
    Ok(())
}

fn cpi_jupiter<'account>(
    jupiter_program: &AccountInfo<'account>,
    remaining: &[AccountInfo<'account>],
    jupiter_data: &[u8],
    vault_authority: &AccountInfo<'account>,
    strategy: &Pubkey,
    vault_bump: u8,
    protected_accounts: &[&Pubkey],
    outer_signers: &[&Pubkey],
) -> ProgramResult {
    if remaining.is_empty() {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    for account in remaining {
        if protected_accounts.contains(&account.key) {
            return Err(StrategySpendError::InvalidAccount.into());
        }
        if account.is_signer
            && account.key != vault_authority.key
            && !outer_signers.contains(&account.key)
        {
            return Err(StrategySpendError::InvalidAccount.into());
        }
    }

    let instruction = solana_program::instruction::Instruction {
        program_id: *jupiter_program.key,
        accounts: remaining
            .iter()
            .map(|account| solana_program::instruction::AccountMeta {
                pubkey: *account.key,
                is_signer: account.key == vault_authority.key,
                is_writable: account.is_writable,
            })
            .collect(),
        data: jupiter_data.to_vec(),
    };
    let mut infos = remaining.to_vec();
    infos.push(jupiter_program.clone());
    invoke_signed(
        &instruction,
        &infos,
        &[&[VAULT_SEED, strategy.as_ref(), &[vault_bump]]],
    )
    .map_err(|_| StrategySpendError::JupiterCpiFailed)?;
    Ok(())
}

fn associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[wallet.as_ref(), spl_token::id().as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
    .0
}

fn create_associated_token_account_idempotent(
    payer: &Pubkey,
    wallet: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: ASSOCIATED_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(
                Pubkey::find_program_address(
                    &[wallet.as_ref(), token_program.as_ref(), mint.as_ref()],
                    &ASSOCIATED_TOKEN_PROGRAM_ID,
                )
                .0,
                false,
            ),
            AccountMeta::new_readonly(*wallet, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(*token_program, false),
        ],
        data: vec![1],
    }
}

#[allow(clippy::too_many_arguments)]
fn ensure_vault_ata<'a>(
    payer: &AccountInfo<'a>,
    vault: &AccountInfo<'a>,
    vault_authority: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    associated_token_program: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
) -> ProgramResult {
    let expected = associated_token_address(vault_authority.key, mint.key);
    if vault.key != &expected {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    if vault.data_is_empty() {
        invoke(
            &create_associated_token_account_idempotent(
                payer.key,
                vault_authority.key,
                mint.key,
                token_program.key,
            ),
            &[
                payer.clone(),
                vault.clone(),
                vault_authority.clone(),
                mint.clone(),
                system_program_account.clone(),
                token_program.clone(),
                associated_token_program.clone(),
            ],
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn transfer_from_vault<'a>(
    token_program: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    vault_authority: &AccountInfo<'a>,
    strategy: &AccountInfo<'a>,
    vault_bump: u8,
    amount: u64,
) -> ProgramResult {
    invoke_signed(
        &token_instruction::transfer_checked(
            token_program.key,
            source.key,
            mint.key,
            destination.key,
            vault_authority.key,
            &[],
            amount,
            mint_decimals(mint)?,
        )?,
        &[
            source.clone(),
            mint.clone(),
            destination.clone(),
            vault_authority.clone(),
            token_program.clone(),
        ],
        &[&[VAULT_SEED, strategy.key.as_ref(), &[vault_bump]]],
    )
}

fn apply_realized_pnl(capacity: u64, limit: u64, realized_pnl: i64) -> Result<u64, ProgramError> {
    let next = i128::from(capacity)
        .checked_add(i128::from(realized_pnl))
        .ok_or(StrategySpendError::Overflow)?;
    u64::try_from(next.clamp(0, i128::from(limit))).map_err(|_| StrategySpendError::Overflow.into())
}

fn pro_rata_cost(total_cost: u64, total_qty: u64, sold_qty: u64) -> Result<u64, ProgramError> {
    if sold_qty == 0 || total_qty == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }
    if sold_qty == total_qty {
        return Ok(total_cost);
    }
    let cost = (total_cost as u128)
        .checked_mul(sold_qty as u128)
        .ok_or(StrategySpendError::Overflow)?
        .checked_div(total_qty as u128)
        .ok_or(StrategySpendError::Overflow)?;
    u64::try_from(cost).map_err(|_| StrategySpendError::Overflow.into())
}

fn assert_active_strategy(strategy: &StrategyAccount, session: &Pubkey) -> ProgramResult {
    if strategy.session != *session {
        return Err(StrategySpendError::SessionMismatch.into());
    }
    if strategy.revoked {
        return Err(StrategySpendError::Revoked.into());
    }
    let now = Clock::get()?.unix_timestamp;
    if strategy.expires_at <= now {
        return Err(StrategySpendError::Expired.into());
    }
    Ok(())
}

fn load_wallet<'a>(
    program_id: &Pubkey,
    owner: &Pubkey,
    wallet: &AccountInfo<'a>,
) -> Result<WalletConfig, ProgramError> {
    let (expected, _) = Pubkey::find_program_address(&[WALLET_SEED, owner.as_ref()], program_id);
    if wallet.key != &expected || wallet.owner != program_id {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let config = WalletConfig::try_from_slice(&wallet.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount)?;
    Ok(config)
}

fn load_strategy(
    program_id: &Pubkey,
    strategy: &AccountInfo,
) -> Result<StrategyAccount, ProgramError> {
    if strategy.owner != program_id {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    StrategyAccount::try_from_slice(&strategy.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount.into())
}

fn load_or_default_asset(asset: &AccountInfo) -> Result<StrategyAsset, ProgramError> {
    if asset.data_is_empty() {
        return Ok(StrategyAsset {
            quantity: 0,
            cost_usdc: 0,
        });
    }
    StrategyAsset::try_from_slice(&asset.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount.into())
}

fn ensure_asset_account<'a>(
    asset: &AccountInfo<'a>,
    strategy: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    bump: u8,
) -> ProgramResult {
    if !asset.data_is_empty() {
        return Ok(());
    }
    create_pda(
        payer,
        asset,
        system_program_account,
        program_id,
        StrategyAsset::LEN,
        &[
            ASSET_SEED,
            strategy.key.as_ref(),
            mint.key.as_ref(),
            &[bump],
        ],
    )?;
    StrategyAsset {
        quantity: 0,
        cost_usdc: 0,
    }
    .serialize(&mut &mut asset.data.borrow_mut()[..])?;
    Ok(())
}

fn assert_strategy_vault(
    vault: &AccountInfo,
    vault_authority: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> ProgramResult {
    if vault.owner != token_program {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let data = TokenAccount::unpack(&vault.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount)?;
    if data.owner != *vault_authority || data.mint != *mint {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    Ok(())
}

fn assert_native_vault(
    vault: &AccountInfo,
    vault_authority: &Pubkey,
    token_program: &Pubkey,
) -> ProgramResult {
    assert_strategy_vault(
        vault,
        vault_authority,
        &spl_token::native_mint::id(),
        token_program,
    )?;
    let data = TokenAccount::unpack(&vault.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount)?;
    if data.is_native.is_none() {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    Ok(())
}

fn assert_usdc_account(account: &AccountInfo, owner: &Pubkey, usdc_mint: &Pubkey) -> ProgramResult {
    let data = TokenAccount::unpack(&account.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount)?;
    if data.owner != *owner || data.mint != *usdc_mint {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    Ok(())
}

fn assert_system_program(system_program_account: &AccountInfo) -> ProgramResult {
    if system_program_account.key != &system_program::id() {
        return Err(StrategySpendError::ProgramMismatch.into());
    }
    Ok(())
}

fn assert_token_program(token_program: &AccountInfo, wallet: &WalletConfig) -> ProgramResult {
    if token_program.key != &spl_token::id() || wallet.token_program != spl_token::id() {
        return Err(StrategySpendError::ProgramMismatch.into());
    }
    Ok(())
}

fn assert_associated_token_program(
    associated_token_program: &AccountInfo,
    wallet: &WalletConfig,
) -> ProgramResult {
    if associated_token_program.key != &wallet.ata_program {
        return Err(StrategySpendError::ProgramMismatch.into());
    }
    Ok(())
}

fn mint_decimals(mint: &AccountInfo) -> Result<u8, ProgramError> {
    let data = Mint::unpack(&mint.data.borrow()).map_err(|_| StrategySpendError::MintMismatch)?;
    Ok(data.decimals)
}

fn token_account_amount(account: &AccountInfo) -> Result<u64, ProgramError> {
    let data = TokenAccount::unpack(&account.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount)?;
    Ok(data.amount)
}

fn native_token_amount(account: &AccountInfo) -> Result<u64, ProgramError> {
    let data = TokenAccount::unpack(&account.data.borrow())
        .map_err(|_| StrategySpendError::InvalidAccount)?;
    if data.mint != spl_token::native_mint::id() || data.is_native.is_none() {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    Ok(data.amount)
}

fn emit_gas_credit(mode: &str, strategy: &Pubkey, recipient: &Pubkey, lamports: u64) {
    msg!(
        "POCKLESS_GAS_CREDIT_V1:{}:{}:{}:{}",
        mode,
        strategy,
        recipient,
        lamports
    );
}

fn scale_to_mint_atomic(amount_usdc: u64, mint: &AccountInfo) -> Result<u64, ProgramError> {
    let decimals = mint_decimals(mint)?;
    if decimals < 6 {
        return Err(StrategySpendError::MintMismatch.into());
    }
    let factor = 10u64
        .checked_pow((decimals - 6) as u32)
        .ok_or(StrategySpendError::Overflow)?;
    amount_usdc
        .checked_mul(factor)
        .ok_or_else(|| StrategySpendError::Overflow.into())
}

fn normalize_from_mint_atomic(amount: u64, mint: &AccountInfo) -> Result<u64, ProgramError> {
    let decimals = mint_decimals(mint)?;
    if decimals < 6 {
        return Err(StrategySpendError::MintMismatch.into());
    }
    let factor = 10u64
        .checked_pow((decimals - 6) as u32)
        .ok_or(StrategySpendError::Overflow)?;
    amount
        .checked_div(factor)
        .ok_or_else(|| StrategySpendError::Overflow.into())
}

fn migrate_wallet_account<'a>(
    program_id: &Pubkey,
    owner: &AccountInfo<'a>,
    wallet: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
) -> ProgramResult {
    if wallet.owner != program_id {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    let data = wallet.data.borrow();
    if data.len() == WalletConfig::LEN {
        if WalletConfig::try_from_slice(&data)
            .ok()
            .is_some_and(|config| config.owner == *owner.key)
        {
            return Err(StrategySpendError::AlreadyInitialized.into());
        }
        return Err(StrategySpendError::InvalidAccount.into());
    }
    if !is_legacy_v1_wallet(&data, owner.key) {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    drop(data);

    let rent = Rent::get()?.minimum_balance(WalletConfig::LEN);
    let deficit = rent.saturating_sub(wallet.lamports());
    if deficit > 0 {
        invoke(
            &system_instruction::transfer(owner.key, wallet.key, deficit),
            &[
                owner.clone(),
                wallet.clone(),
                system_program_account.clone(),
            ],
        )?;
    }
    wallet.realloc(WalletConfig::LEN, false)?;
    Ok(())
}

fn is_legacy_v1_wallet(data: &[u8], owner: &Pubkey) -> bool {
    if data.len() != WalletConfig::LEGACY_V1_LEN || data[0] != WalletConfig::LEGACY_V1_VERSION {
        return false;
    }
    data.get(1..33).is_some_and(|bytes| bytes == owner.as_ref())
}

fn create_pda<'a>(
    payer: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    space: usize,
    seeds: &[&[u8]],
) -> ProgramResult {
    let rent = Rent::get()?.minimum_balance(space);
    invoke_signed(
        &system_instruction::create_account(payer.key, pda.key, rent, space as u64, program_id),
        &[payer.clone(), pda.clone(), system_program_account.clone()],
        &[seeds],
    )
}
