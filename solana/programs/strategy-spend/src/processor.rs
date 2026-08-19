use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
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
use crate::instruction::StrategySpendInstruction;
use crate::state::{
    StrategyAccount, StrategyAsset, WalletConfig, ASSET_SEED, AUTHORITY_SEED, STRATEGY_SEED,
    VAULT_SEED, WALLET_CONFIG_VERSION, WALLET_SEED,
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
        StrategySpendInstruction::ExecuteSwap {
            is_buy,
            usdc_amount,
            token_amount,
            jupiter_data,
        } => execute_swap(
            program_id,
            accounts,
            is_buy,
            usdc_amount,
            token_amount,
            jupiter_data,
        ),
        StrategySpendInstruction::WithdrawAsset { amount } => {
            withdraw_asset(program_id, accounts, amount)
        }
        StrategySpendInstruction::CloseStrategy => close_strategy(program_id, accounts),
    }
}

fn init_wallet(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let usdc_mint = next_account_info(account_iter)?;
    let jupiter_program = next_account_info(account_iter)?;
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
    if !wallet.data_is_empty() {
        return Err(StrategySpendError::AlreadyInitialized.into());
    }

    let (_, authority_bump) =
        Pubkey::find_program_address(&[AUTHORITY_SEED, owner.key.as_ref()], program_id);

    create_pda(
        owner,
        wallet,
        system_program_account,
        program_id,
        WalletConfig::LEN,
        &[WALLET_SEED, owner.key.as_ref(), &[wallet_bump]],
    )?;

    WalletConfig {
        version: WALLET_CONFIG_VERSION,
        owner: *owner.key,
        usdc_mint: *usdc_mint.key,
        token_program: spl_token::id(),
        associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
        jupiter_program: *jupiter_program.key,
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
    state.serialize(&mut &mut strategy.data.borrow_mut()[..])?;
    Ok(())
}

fn execute_swap(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    is_buy: bool,
    usdc_amount: u64,
    token_amount: u64,
    jupiter_data: Vec<u8>,
) -> ProgramResult {
    if jupiter_data.is_empty() || usdc_amount == 0 || token_amount == 0 {
        return Err(StrategySpendError::InvalidInstruction.into());
    }

    let account_iter = &mut accounts.iter();
    let session = next_account_info(account_iter)?;
    let relayer = next_account_info(account_iter)?;
    let owner = next_account_info(account_iter)?;
    let wallet = next_account_info(account_iter)?;
    let strategy = next_account_info(account_iter)?;
    let vault_authority = next_account_info(account_iter)?;
    let owner_usdc = next_account_info(account_iter)?;
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

    if !session.is_signer || !relayer.is_signer {
        return Err(StrategySpendError::MissingSignature.into());
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

    let protected_accounts = [
        session.key,
        relayer.key,
        owner.key,
        wallet.key,
        strategy.key,
        owner_usdc.key,
        asset_account.key,
        program_authority.key,
    ];

    if is_buy {
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
            account_iter,
            &jupiter_data,
            vault_authority,
            strategy.key,
            vault_bump,
            &protected_accounts,
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
    } else {
        let mut asset = load_or_default_asset(asset_account)?;
        if asset.quantity < token_amount {
            return Err(StrategySpendError::InsufficientAsset.into());
        }

        let token_before = token_account_amount(strategy_token_vault)?;
        let strategy_usdc_before = token_account_amount(strategy_usdc)?;
        cpi_jupiter(
            jupiter_program,
            account_iter,
            &jupiter_data,
            vault_authority,
            strategy.key,
            vault_bump,
            &protected_accounts,
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
    }

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

fn cpi_jupiter<'slice, 'account>(
    jupiter_program: &AccountInfo<'account>,
    account_iter: &mut std::slice::Iter<'slice, AccountInfo<'account>>,
    jupiter_data: &[u8],
    vault_authority: &AccountInfo<'account>,
    strategy: &Pubkey,
    vault_bump: u8,
    protected_accounts: &[&Pubkey],
) -> ProgramResult {
    let remaining: Vec<AccountInfo> = account_iter.cloned().collect();
    if remaining.is_empty() {
        return Err(StrategySpendError::InvalidAccount.into());
    }
    for account in &remaining {
        if protected_accounts.contains(&account.key)
            || (account.is_signer && account.key != vault_authority.key)
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
    let mut infos = remaining;
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
    if config.version != WALLET_CONFIG_VERSION {
        return Err(StrategySpendError::InvalidAccount.into());
    }
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
    if associated_token_program.key != &ASSOCIATED_TOKEN_PROGRAM_ID
        || wallet.associated_token_program != ASSOCIATED_TOKEN_PROGRAM_ID
    {
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
