use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_option::COption,
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction, system_program,
};
use solana_program_test::{processor, BanksClientError, ProgramTest};
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount, AccountState, Mint},
};
use strategy_spend::instruction::{GasMode, StrategySpendInstruction};
use strategy_spend::state::{
    RemoteMintAggregate, RemoteStrategyAsset, StrategyAccount, StrategyAsset, WalletConfig,
    ASSET_SEED, AUTHORITY_SEED, RELAY_ACTION_CREDIT_ASSET, RELAY_ACTION_DEPOSIT,
    RELAY_ACTION_REMOTE_SELL, RELAY_ACTION_USDC_RETURN, RELAY_PENDING_DEPOSIT_SEED,
    RELAY_PENDING_SELL_SEED, RELAY_RECEIPT_SEED, REMOTE_AGGREGATE_SEED, REMOTE_ASSET_SEED,
    SOLANA_RELAY_CHAIN_ID, STRATEGY_SEED, VAULT_SEED, WALLET_SEED,
};

const LIMIT_USDC: u64 = 1_000_000_000;
const EXPIRES_AT: i64 = 4_102_444_800;
const GAS_FUNDER_SEED: &[u8] = b"gas-funder";
const FORWARDER_SEED: &[u8] = b"forwarder";

fn strategy_id(seed: &str) -> [u8; 32] {
    solana_sdk::hash::hash(seed.as_bytes()).to_bytes()
}

fn wallet_pda(
    program_id: &solana_sdk::pubkey::Pubkey,
    owner: &solana_sdk::pubkey::Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(&[WALLET_SEED, owner.as_ref()], program_id).0
}

fn strategy_pda(
    program_id: &solana_sdk::pubkey::Pubkey,
    owner: &solana_sdk::pubkey::Pubkey,
    strategy_id: &[u8; 32],
) -> Pubkey {
    Pubkey::find_program_address(
        &[STRATEGY_SEED, owner.as_ref(), strategy_id.as_ref()],
        program_id,
    )
    .0
}

fn mint_account(decimals: u8, authority: Pubkey, supply: u64) -> Account {
    let mint = Mint {
        mint_authority: COption::Some(authority),
        supply,
        decimals,
        is_initialized: true,
        freeze_authority: None.into(),
    };
    let mut data = vec![0u8; Mint::LEN];
    Mint::pack(mint, &mut data).unwrap();
    Account {
        lamports: 1_000_000_000,
        data,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: 0,
    }
}

fn token_account(mint: Pubkey, owner: Pubkey, amount: u64, delegate: Option<Pubkey>) -> Account {
    let token = TokenAccount {
        mint,
        owner,
        amount,
        delegate: delegate.map(COption::Some).unwrap_or(COption::None),
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: if delegate.is_some() { u64::MAX } else { 0 },
        close_authority: COption::None,
    };
    let mut data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(token, &mut data).unwrap();
    Account {
        lamports: 1_000_000_000,
        data,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: 0,
    }
}

fn native_token_account(owner: Pubkey, reserve: u64) -> Account {
    let token = TokenAccount {
        mint: spl_token::native_mint::id(),
        owner,
        amount: 0,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::Some(reserve),
        delegated_amount: 0,
        close_authority: COption::None,
    };
    let mut data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(token, &mut data).unwrap();
    Account {
        lamports: reserve,
        data,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: 0,
    }
}

fn mock_jupiter(_program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() == 7
        && accounts[3].key == &Pubkey::find_program_address(&[GAS_FUNDER_SEED], _program_id).0
    {
        mock_gas_swap(_program_id, accounts, data)
    } else {
        mock_token_swap(_program_id, accounts, data)
    }
}

fn mock_gas_swap(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let authority = next_account_info(accounts)?;
    let source = next_account_info(accounts)?;
    let gas_wsol = next_account_info(accounts)?;
    let gas_funder = next_account_info(accounts)?;
    let input_mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;
    let system_program_account = next_account_info(accounts)?;
    let input_amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let lamports_out = u64::from_le_bytes(data[8..16].try_into().unwrap());

    invoke(
        &token_instruction::burn_checked(
            token_program.key,
            source.key,
            input_mint.key,
            authority.key,
            &[],
            input_amount,
            6,
        )?,
        &[
            source.clone(),
            input_mint.clone(),
            authority.clone(),
            token_program.clone(),
        ],
    )?;
    let (_, gas_funder_bump) = Pubkey::find_program_address(&[GAS_FUNDER_SEED], program_id);
    invoke_signed(
        &system_instruction::transfer(gas_funder.key, gas_wsol.key, lamports_out),
        &[
            gas_funder.clone(),
            gas_wsol.clone(),
            system_program_account.clone(),
        ],
        &[&[GAS_FUNDER_SEED, &[gas_funder_bump]]],
    )?;
    invoke(
        &token_instruction::sync_native(token_program.key, gas_wsol.key)?,
        &[gas_wsol.clone(), token_program.clone()],
    )
}

fn mock_token_swap(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let authority = next_account_info(accounts)?;
    let source = next_account_info(accounts)?;
    let destination = next_account_info(accounts)?;
    let input_mint = next_account_info(accounts)?;
    let output_mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;
    let input_amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let output_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());

    if input_mint.key == &spl_token::native_mint::id() {
        // Wrapped SOL cannot be burned, so the sold side moves to a sink.
        let input_sink = next_account_info(accounts)?;
        invoke(
            &token_instruction::transfer_checked(
                token_program.key,
                source.key,
                input_mint.key,
                input_sink.key,
                authority.key,
                &[],
                input_amount,
                9,
            )?,
            &[
                source.clone(),
                input_mint.clone(),
                input_sink.clone(),
                authority.clone(),
                token_program.clone(),
            ],
        )?;
    } else {
        invoke(
            &token_instruction::burn_checked(
                token_program.key,
                source.key,
                input_mint.key,
                authority.key,
                &[],
                input_amount,
                6,
            )?,
            &[
                source.clone(),
                input_mint.clone(),
                authority.clone(),
                token_program.clone(),
            ],
        )?;
    }
    if output_mint.key == &spl_token::native_mint::id() {
        let gas_funder = next_account_info(accounts)?;
        let system_program_account = next_account_info(accounts)?;
        let (_, gas_funder_bump) = Pubkey::find_program_address(&[GAS_FUNDER_SEED], program_id);
        invoke_signed(
            &system_instruction::transfer(gas_funder.key, destination.key, output_amount),
            &[
                gas_funder.clone(),
                destination.clone(),
                system_program_account.clone(),
            ],
            &[&[GAS_FUNDER_SEED, &[gas_funder_bump]]],
        )?;
        invoke(
            &token_instruction::sync_native(token_program.key, destination.key)?,
            &[destination.clone(), token_program.clone()],
        )
    } else {
        invoke(
            &token_instruction::mint_to_checked(
                token_program.key,
                output_mint.key,
                destination.key,
                authority.key,
                &[],
                output_amount,
                6,
            )?,
            &[
                output_mint.clone(),
                destination.clone(),
                authority.clone(),
                token_program.clone(),
            ],
        )
    }
}

/// Stands in for Relay's `relay_forwarder`: `forward_token` takes no amount and
/// sweeps whatever the origin swap left in the forwarder's token account.
fn mock_forwarder(program_id: &Pubkey, accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let sender = next_account_info(accounts)?;
    let forwarder = next_account_info(accounts)?;
    let forwarder_token_account = next_account_info(accounts)?;
    let relay_vault_token_account = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;
    if !sender.is_signer {
        return Err(solana_program::program_error::ProgramError::MissingRequiredSignature);
    }
    let amount = TokenAccount::unpack(&forwarder_token_account.data.borrow())?.amount;
    let (_, bump) = Pubkey::find_program_address(&[FORWARDER_SEED], program_id);
    invoke_signed(
        &token_instruction::transfer_checked(
            token_program.key,
            forwarder_token_account.key,
            mint.key,
            relay_vault_token_account.key,
            forwarder.key,
            &[],
            amount,
            6,
        )?,
        &[
            forwarder_token_account.clone(),
            mint.clone(),
            relay_vault_token_account.clone(),
            forwarder.clone(),
            token_program.clone(),
        ],
        &[&[FORWARDER_SEED, &[bump]]],
    )
}

/// Mirrors `relay_depository::deposit_token`: the sender signs, and the vault
/// USDC it owns is pulled into the relay vault. The sender's signature reaches
/// the token transfer through CPI privilege inheritance, which is the whole
/// reason a keyless vault can deposit at all.
fn mock_relay_depository(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let _depository = next_account_info(accounts)?;
    let sender = next_account_info(accounts)?;
    let _depositor = next_account_info(accounts)?;
    let _relay_vault = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let sender_token_account = next_account_info(accounts)?;
    let vault_token_account = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;
    if !sender.is_signer {
        return Err(solana_program::program_error::ProgramError::MissingRequiredSignature);
    }
    let amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
    invoke(
        &token_instruction::transfer_checked(
            token_program.key,
            sender_token_account.key,
            mint.key,
            vault_token_account.key,
            sender.key,
            &[],
            amount,
            6,
        )?,
        &[
            sender_token_account.clone(),
            mint.clone(),
            vault_token_account.clone(),
            sender.clone(),
            token_program.clone(),
        ],
    )
}

struct TestHarness {
    program_id: Pubkey,
    jupiter_program: Pubkey,
    relay_depository: Pubkey,
    owner: Keypair,
    session: Keypair,
    relayer: Keypair,
    treasury: Keypair,
    usdc_mint: Pubkey,
    token_mint: Pubkey,
    gas_funder: Pubkey,
    native_output_wsol: Pubkey,
    forwarder: Pubkey,
    strategy_id: [u8; 32],
}

impl TestHarness {
    async fn start() -> (Self, solana_program_test::BanksClient, Keypair) {
        Self::start_with(false).await
    }

    async fn start_with_legacy_v1_wallet() -> (Self, solana_program_test::BanksClient, Keypair) {
        Self::start_with(true).await
    }

    async fn start_with(
        legacy_v1_wallet: bool,
    ) -> (Self, solana_program_test::BanksClient, Keypair) {
        let program_id = Pubkey::new_unique();
        let jupiter_program = Pubkey::new_unique();
        let relay_depository = Pubkey::new_unique();
        let owner = Keypair::new();
        let session = Keypair::new();
        let relayer = Keypair::new();
        let treasury = Keypair::new();
        let usdc_mint = Pubkey::new_unique();
        let token_mint = Pubkey::new_unique();
        let gas_funder = Pubkey::find_program_address(&[GAS_FUNDER_SEED], &jupiter_program).0;
        let native_output_wsol = Pubkey::new_unique();
        let strategy_id = strategy_id("strategy-a");
        let strategy = strategy_pda(&program_id, &owner.pubkey(), &strategy_id);
        let vault_authority =
            Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &program_id).0;
        let program_authority =
            Pubkey::find_program_address(&[AUTHORITY_SEED, owner.pubkey().as_ref()], &program_id).0;
        let mut program_test = ProgramTest::new(
            "strategy_spend",
            program_id,
            processor!(strategy_spend::process_instruction),
        );
        program_test.add_program("mock_jupiter", jupiter_program, processor!(mock_jupiter));
        program_test.add_program(
            "mock_relay_depository",
            relay_depository,
            processor!(mock_relay_depository),
        );
        program_test.add_program(
            "mock_forwarder",
            strategy_spend::processor::RELAY_FORWARDER_PROGRAM_ID,
            processor!(mock_forwarder),
        );
        program_test.add_program(
            "spl_token",
            spl_token::id(),
            processor!(spl_token::processor::Processor::process),
        );
        program_test.add_program(
            "spl_associated_token_account",
            spl_associated_token_account::id(),
            processor!(spl_associated_token_account::processor::process_instruction),
        );
        program_test.add_account(usdc_mint, mint_account(6, vault_authority, LIMIT_USDC));
        program_test.add_account(token_mint, mint_account(6, vault_authority, 0));
        program_test.add_account(
            spl_token::native_mint::id(),
            mint_account(9, vault_authority, 0),
        );
        program_test.add_account(
            get_associated_token_address(&vault_authority, &spl_token::native_mint::id()),
            native_token_account(vault_authority, 1_000_000_000),
        );
        program_test.add_account(
            gas_funder,
            Account {
                lamports: 1_000_000_000,
                data: vec![],
                owner: system_program::id(),
                executable: false,
                rent_epoch: 0,
            },
        );
        program_test.add_account(
            native_output_wsol,
            native_token_account(vault_authority, 2_000_000_000),
        );
        let forwarder = Pubkey::find_program_address(
            &[FORWARDER_SEED],
            &strategy_spend::processor::RELAY_FORWARDER_PROGRAM_ID,
        )
        .0;
        program_test.add_account(
            get_associated_token_address(&forwarder, &usdc_mint),
            token_account(usdc_mint, forwarder, 0, None),
        );
        program_test.add_account(
            get_associated_token_address(
                &strategy_spend::processor::RELAY_FORWARDER_PROGRAM_ID,
                &usdc_mint,
            ),
            token_account(
                usdc_mint,
                strategy_spend::processor::RELAY_FORWARDER_PROGRAM_ID,
                0,
                None,
            ),
        );
        program_test.add_account(
            get_associated_token_address(&owner.pubkey(), &usdc_mint),
            token_account(
                usdc_mint,
                owner.pubkey(),
                LIMIT_USDC,
                Some(program_authority),
            ),
        );
        program_test.add_account(
            get_associated_token_address(&treasury.pubkey(), &usdc_mint),
            token_account(usdc_mint, treasury.pubkey(), 0, None),
        );
        program_test.add_account(
            get_associated_token_address(&vault_authority, &usdc_mint),
            token_account(usdc_mint, vault_authority, 0, None),
        );
        let relay_depository_vault =
            Pubkey::find_program_address(&[VAULT_SEED], &relay_depository).0;
        program_test.add_account(
            get_associated_token_address(&relay_depository_vault, &usdc_mint),
            token_account(usdc_mint, relay_depository_vault, 0, None),
        );
        program_test.add_account(
            get_associated_token_address(&owner.pubkey(), &token_mint),
            token_account(token_mint, owner.pubkey(), 0, None),
        );
        program_test.add_account(
            owner.pubkey(),
            Account {
                lamports: 10_000_000_000,
                data: vec![],
                owner: system_program::id(),
                executable: false,
                rent_epoch: 0,
            },
        );
        if legacy_v1_wallet {
            let wallet = wallet_pda(&program_id, &owner.pubkey());
            let (_, authority_bump) = Pubkey::find_program_address(
                &[AUTHORITY_SEED, owner.pubkey().as_ref()],
                &program_id,
            );
            let mut data = vec![WalletConfig::LEGACY_V1_VERSION];
            data.extend_from_slice(owner.pubkey().as_ref());
            data.extend_from_slice(usdc_mint.as_ref());
            data.extend_from_slice(spl_token::id().as_ref());
            data.extend_from_slice(spl_associated_token_account::id().as_ref());
            data.extend_from_slice(jupiter_program.as_ref());
            data.push(authority_bump);
            program_test.add_account(
                wallet,
                Account {
                    lamports: 2_018_400,
                    data,
                    owner: program_id,
                    executable: false,
                    rent_epoch: 0,
                },
            );
        }
        program_test.add_account(
            relayer.pubkey(),
            Account {
                lamports: 10_000_000_000,
                data: vec![],
                owner: system_program::id(),
                executable: false,
                rent_epoch: 0,
            },
        );

        let (banks_client, payer, _) = program_test.start().await;
        let harness = Self {
            program_id,
            jupiter_program,
            relay_depository,
            owner,
            session,
            relayer,
            treasury,
            usdc_mint,
            token_mint,
            gas_funder,
            native_output_wsol,
            forwarder,
            strategy_id,
        };
        (harness, banks_client, payer)
    }

    fn init_wallet_ix(&self) -> Instruction {
        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(self.owner.pubkey(), true),
                AccountMeta::new(wallet_pda(&self.program_id, &self.owner.pubkey()), false),
                AccountMeta::new_readonly(self.usdc_mint, false),
                AccountMeta::new_readonly(self.jupiter_program, false),
                AccountMeta::new_readonly(self.relay_depository, false),
                AccountMeta::new_readonly(self.relayer.pubkey(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: StrategySpendInstruction::InitWallet.try_to_vec().unwrap(),
        }
    }

    fn init_strategy_ix(&self) -> Instruction {
        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(self.owner.pubkey(), true),
                AccountMeta::new_readonly(
                    wallet_pda(&self.program_id, &self.owner.pubkey()),
                    false,
                ),
                AccountMeta::new(
                    strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id),
                    false,
                ),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: StrategySpendInstruction::InitStrategy {
                strategy_id: self.strategy_id,
                session: self.session.pubkey(),
                limit_usdc: LIMIT_USDC,
                expires_at: EXPIRES_AT,
            }
            .try_to_vec()
            .unwrap(),
        }
    }

    fn set_limit_ix(&self, limit_usdc: u64, expires_at: i64) -> Instruction {
        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(self.owner.pubkey(), true),
                AccountMeta::new_readonly(
                    wallet_pda(&self.program_id, &self.owner.pubkey()),
                    false,
                ),
                AccountMeta::new(
                    strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id),
                    false,
                ),
            ],
            data: StrategySpendInstruction::SetLimit {
                limit_usdc,
                expires_at,
            }
            .try_to_vec()
            .unwrap(),
        }
    }

    fn revoke_ix(&self, authority: &Keypair) -> Instruction {
        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(authority.pubkey(), true),
                AccountMeta::new(
                    strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id),
                    false,
                ),
            ],
            data: StrategySpendInstruction::Revoke.try_to_vec().unwrap(),
        }
    }

    fn rotate_session_ix(&self, new_session: &Pubkey) -> Instruction {
        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(self.owner.pubkey(), true),
                AccountMeta::new_readonly(
                    wallet_pda(&self.program_id, &self.owner.pubkey()),
                    false,
                ),
                AccountMeta::new(
                    strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id),
                    false,
                ),
            ],
            data: StrategySpendInstruction::RotateSession {
                new_session: *new_session,
            }
            .try_to_vec()
            .unwrap(),
        }
    }

    fn close_strategy_ix(&self) -> Instruction {
        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(self.owner.pubkey(), true),
                AccountMeta::new_readonly(
                    wallet_pda(&self.program_id, &self.owner.pubkey()),
                    false,
                ),
                AccountMeta::new(
                    strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id),
                    false,
                ),
                AccountMeta::new(self.owner.pubkey(), false),
            ],
            data: StrategySpendInstruction::CloseStrategy
                .try_to_vec()
                .unwrap(),
        }
    }

    fn execute_swap_with_fees_ix(
        &self,
        is_buy: bool,
        usdc_amount: u64,
        token_amount: u64,
        platform_fee_usdc: u64,
        gas_mode: GasMode,
        gas_top_up_usdc: u64,
        native_amount: u64,
        actual_input: u64,
        actual_output: u64,
        gas_input: u64,
        gas_lamports_out: u64,
    ) -> Instruction {
        let strategy = strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id);
        let vault_authority =
            Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &self.program_id).0;
        let program_authority = Pubkey::find_program_address(
            &[AUTHORITY_SEED, self.owner.pubkey().as_ref()],
            &self.program_id,
        )
        .0;
        let owner_usdc = get_associated_token_address(&self.owner.pubkey(), &self.usdc_mint);
        let treasury_usdc = get_associated_token_address(&self.treasury.pubkey(), &self.usdc_mint);
        let strategy_usdc = get_associated_token_address(&vault_authority, &self.usdc_mint);
        let strategy_token = get_associated_token_address(&vault_authority, &self.token_mint);
        let asset = Pubkey::find_program_address(
            &[ASSET_SEED, strategy.as_ref(), self.token_mint.as_ref()],
            &self.program_id,
        )
        .0;
        let (source, destination, input_mint, output_mint) = if is_buy {
            (
                strategy_usdc,
                strategy_token,
                self.usdc_mint,
                self.token_mint,
            )
        } else {
            (
                strategy_token,
                strategy_usdc,
                self.token_mint,
                self.usdc_mint,
            )
        };
        let mut jupiter_data = actual_input.to_le_bytes().to_vec();
        jupiter_data.extend_from_slice(&actual_output.to_le_bytes());
        let mut gas_jupiter_data = gas_input.to_le_bytes().to_vec();
        gas_jupiter_data.extend_from_slice(&gas_lamports_out.to_le_bytes());
        let gas_payload = if gas_mode == GasMode::Separate {
            let mut payload = vec![7u8];
            payload.extend_from_slice(&gas_jupiter_data);
            payload
        } else {
            vec![]
        };

        let mut accounts = vec![
            AccountMeta::new_readonly(self.session.pubkey(), true),
            AccountMeta::new(self.relayer.pubkey(), true),
            AccountMeta::new_readonly(self.owner.pubkey(), false),
            AccountMeta::new_readonly(wallet_pda(&self.program_id, &self.owner.pubkey()), false),
            AccountMeta::new(strategy, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new(owner_usdc, false),
            AccountMeta::new(treasury_usdc, false),
            AccountMeta::new(strategy_usdc, false),
            AccountMeta::new(strategy_token, false),
            AccountMeta::new(asset, false),
            AccountMeta::new_readonly(self.token_mint, false),
            AccountMeta::new_readonly(self.usdc_mint, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(program_authority, false),
            AccountMeta::new_readonly(self.jupiter_program, false),
            if gas_mode == GasMode::Separate {
                AccountMeta::new(
                    get_associated_token_address(&vault_authority, &spl_token::native_mint::id()),
                    false,
                )
            } else {
                AccountMeta::new(self.relayer.pubkey(), false)
            },
        ];
        if gas_mode == GasMode::Separate {
            accounts.extend_from_slice(&[
                AccountMeta::new_readonly(vault_authority, false),
                AccountMeta::new(strategy_usdc, false),
                AccountMeta::new(
                    get_associated_token_address(&vault_authority, &spl_token::native_mint::id()),
                    false,
                ),
                AccountMeta::new(self.gas_funder, false),
                AccountMeta::new(self.usdc_mint, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ]);
        }
        accounts.extend_from_slice(&[
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new(source, false),
            AccountMeta::new(destination, false),
            AccountMeta::new(input_mint, false),
            AccountMeta::new(output_mint, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ]);

        Instruction {
            program_id: self.program_id,
            accounts,
            data: StrategySpendInstruction::ExecuteSwapWithFees {
                is_buy,
                usdc_amount,
                token_amount,
                platform_fee_usdc,
                gas_mode,
                gas_top_up_usdc,
                native_amount,
                treasury: self.treasury.pubkey(),
                gas_recipient: self.relayer.pubkey(),
                jupiter_data,
                gas_jupiter_data: gas_payload,
            }
            .try_to_vec()
            .unwrap(),
        }
    }

    fn execute_swap_with_fees_v2_separate_ix(
        &self,
        gas_recipient: Pubkey,
        gas_lamports_out: u64,
        min_native_out: u64,
    ) -> Instruction {
        let mut instruction = self.execute_swap_with_fees_ix(
            true,
            200_000_000,
            90_000_000,
            1_000_000,
            GasMode::Separate,
            500_000,
            min_native_out,
            200_000_000,
            100_000_000,
            500_000,
            gas_lamports_out,
        );
        instruction.data[75..107].copy_from_slice(&gas_recipient.to_bytes());
        instruction
    }

    fn execute_swap_with_fees_v2_credit_only_ix(&self) -> Instruction {
        self.execute_swap_with_fees_ix(
            true,
            200_000_000,
            90_000_000,
            0,
            GasMode::CreditOnly,
            0,
            0,
            200_000_000,
            100_000_000,
            0,
            0,
        )
    }

    /// A native-SOL buy that pays gas from existing credit. Structurally this
    /// is the plain token path with a WSOL output, so the program never takes
    /// the `native_output` branch.
    fn execute_wsol_credit_only_ix(&self) -> Instruction {
        let strategy = strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id);
        let vault_authority =
            Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &self.program_id).0;
        let program_authority = Pubkey::find_program_address(
            &[AUTHORITY_SEED, self.owner.pubkey().as_ref()],
            &self.program_id,
        )
        .0;
        let strategy_usdc = get_associated_token_address(&vault_authority, &self.usdc_mint);
        let strategy_wsol =
            get_associated_token_address(&vault_authority, &spl_token::native_mint::id());
        let asset = Pubkey::find_program_address(
            &[
                ASSET_SEED,
                strategy.as_ref(),
                spl_token::native_mint::id().as_ref(),
            ],
            &self.program_id,
        )
        .0;
        let mut jupiter_data = 200_000_000u64.to_le_bytes().to_vec();
        jupiter_data.extend_from_slice(&100_000_000u64.to_le_bytes());
        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new_readonly(self.session.pubkey(), true),
                AccountMeta::new(self.relayer.pubkey(), true),
                AccountMeta::new_readonly(self.owner.pubkey(), false),
                AccountMeta::new_readonly(
                    wallet_pda(&self.program_id, &self.owner.pubkey()),
                    false,
                ),
                AccountMeta::new(strategy, false),
                AccountMeta::new_readonly(vault_authority, false),
                AccountMeta::new(
                    get_associated_token_address(&self.owner.pubkey(), &self.usdc_mint),
                    false,
                ),
                AccountMeta::new(
                    get_associated_token_address(&self.treasury.pubkey(), &self.usdc_mint),
                    false,
                ),
                AccountMeta::new(strategy_usdc, false),
                AccountMeta::new(strategy_wsol, false),
                AccountMeta::new(asset, false),
                AccountMeta::new_readonly(spl_token::native_mint::id(), false),
                AccountMeta::new_readonly(self.usdc_mint, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(program_authority, false),
                AccountMeta::new_readonly(self.jupiter_program, false),
                // Gas slot is unused when the credit already exists.
                AccountMeta::new(self.relayer.pubkey(), false),
                AccountMeta::new_readonly(vault_authority, false),
                AccountMeta::new(strategy_usdc, false),
                AccountMeta::new(strategy_wsol, false),
                AccountMeta::new(self.usdc_mint, false),
                AccountMeta::new(spl_token::native_mint::id(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new(self.gas_funder, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: StrategySpendInstruction::ExecuteSwapWithFees {
                is_buy: true,
                usdc_amount: 200_000_000,
                token_amount: 90_000_000,
                platform_fee_usdc: 0,
                gas_mode: GasMode::CreditOnly,
                gas_top_up_usdc: 0,
                native_amount: 0,
                treasury: self.treasury.pubkey(),
                gas_recipient: self.relayer.pubkey(),
                jupiter_data,
                gas_jupiter_data: vec![],
            }
            .try_to_vec()
            .unwrap(),
        }
    }

    fn execute_native_output_ix(
        &self,
        gas_recipient: Pubkey,
        native_amount: u64,
        actual_output: u64,
    ) -> Instruction {
        let strategy = strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id);
        let vault_authority =
            Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &self.program_id).0;
        let program_authority = Pubkey::find_program_address(
            &[AUTHORITY_SEED, self.owner.pubkey().as_ref()],
            &self.program_id,
        )
        .0;
        let strategy_usdc = get_associated_token_address(&vault_authority, &self.usdc_mint);
        let strategy_wsol =
            get_associated_token_address(&vault_authority, &spl_token::native_mint::id());
        let asset = Pubkey::find_program_address(
            &[
                ASSET_SEED,
                strategy.as_ref(),
                spl_token::native_mint::id().as_ref(),
            ],
            &self.program_id,
        )
        .0;
        let total_input = 200_500_000u64;
        let mut jupiter_data = total_input.to_le_bytes().to_vec();
        jupiter_data.extend_from_slice(&actual_output.to_le_bytes());
        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new_readonly(self.session.pubkey(), true),
                AccountMeta::new(self.relayer.pubkey(), true),
                AccountMeta::new_readonly(self.owner.pubkey(), false),
                AccountMeta::new_readonly(
                    wallet_pda(&self.program_id, &self.owner.pubkey()),
                    false,
                ),
                AccountMeta::new(strategy, false),
                AccountMeta::new_readonly(vault_authority, false),
                AccountMeta::new(
                    get_associated_token_address(&self.owner.pubkey(), &self.usdc_mint),
                    false,
                ),
                AccountMeta::new(
                    get_associated_token_address(&self.treasury.pubkey(), &self.usdc_mint),
                    false,
                ),
                AccountMeta::new(strategy_usdc, false),
                AccountMeta::new(strategy_wsol, false),
                AccountMeta::new(asset, false),
                AccountMeta::new_readonly(spl_token::native_mint::id(), false),
                AccountMeta::new_readonly(self.usdc_mint, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(program_authority, false),
                AccountMeta::new_readonly(self.jupiter_program, false),
                AccountMeta::new(self.native_output_wsol, false),
                AccountMeta::new_readonly(vault_authority, false),
                AccountMeta::new(strategy_usdc, false),
                AccountMeta::new(strategy_wsol, false),
                AccountMeta::new(self.usdc_mint, false),
                AccountMeta::new(spl_token::native_mint::id(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new(self.gas_funder, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: StrategySpendInstruction::ExecuteSwapWithFees {
                is_buy: true,
                usdc_amount: 200_000_000,
                token_amount: 90_000_000,
                platform_fee_usdc: 0,
                gas_mode: GasMode::NativeOutput,
                gas_top_up_usdc: 500_000,
                native_amount,
                treasury: self.treasury.pubkey(),
                gas_recipient,
                jupiter_data,
                gas_jupiter_data: vec![],
            }
            .try_to_vec()
            .unwrap(),
        }
    }

    fn withdraw_ix(&self, amount: u64) -> Instruction {
        let strategy = strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id);
        let vault_authority =
            Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &self.program_id).0;
        let strategy_token = get_associated_token_address(&vault_authority, &self.token_mint);
        let owner_token = get_associated_token_address(&self.owner.pubkey(), &self.token_mint);
        let asset = Pubkey::find_program_address(
            &[ASSET_SEED, strategy.as_ref(), self.token_mint.as_ref()],
            &self.program_id,
        )
        .0;
        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new_readonly(self.owner.pubkey(), true),
                AccountMeta::new_readonly(
                    wallet_pda(&self.program_id, &self.owner.pubkey()),
                    false,
                ),
                AccountMeta::new(strategy, false),
                AccountMeta::new_readonly(vault_authority, false),
                AccountMeta::new(strategy_token, false),
                AccountMeta::new(owner_token, false),
                AccountMeta::new_readonly(self.token_mint, false),
                AccountMeta::new(asset, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: StrategySpendInstruction::WithdrawAsset { amount }
                .try_to_vec()
                .unwrap(),
        }
    }

    async fn read_strategy(
        &self,
        banks_client: &mut solana_program_test::BanksClient,
    ) -> StrategyAccount {
        let strategy = strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id);
        let account = banks_client.get_account(strategy).await.unwrap().unwrap();
        StrategyAccount::try_from_slice(&account.data).unwrap()
    }

    fn credit_relay_asset_ix(
        &self,
        relay_order_id: [u8; 32],
        funding_chain_id: u64,
        credit_quantity: u64,
        cost_usdc: u64,
    ) -> Instruction {
        let strategy = strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id);
        let vault_authority =
            Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &self.program_id).0;
        let wsol_vault =
            get_associated_token_address(&vault_authority, &spl_token::native_mint::id());
        let remote_asset = Pubkey::find_program_address(
            &[
                REMOTE_ASSET_SEED,
                strategy.as_ref(),
                spl_token::native_mint::id().as_ref(),
                &funding_chain_id.to_le_bytes(),
            ],
            &self.program_id,
        )
        .0;
        let remote_aggregate = Pubkey::find_program_address(
            &[
                REMOTE_AGGREGATE_SEED,
                strategy.as_ref(),
                spl_token::native_mint::id().as_ref(),
            ],
            &self.program_id,
        )
        .0;
        let relay_receipt = Pubkey::find_program_address(
            &[
                RELAY_RECEIPT_SEED,
                strategy.as_ref(),
                relay_order_id.as_ref(),
                &[RELAY_ACTION_CREDIT_ASSET],
            ],
            &self.program_id,
        )
        .0;
        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new_readonly(self.session.pubkey(), true),
                AccountMeta::new(self.relayer.pubkey(), true),
                AccountMeta::new_readonly(self.owner.pubkey(), false),
                AccountMeta::new_readonly(
                    wallet_pda(&self.program_id, &self.owner.pubkey()),
                    false,
                ),
                AccountMeta::new(strategy, false),
                AccountMeta::new(vault_authority, false),
                AccountMeta::new(wsol_vault, false),
                AccountMeta::new_readonly(spl_token::native_mint::id(), false),
                AccountMeta::new(remote_asset, false),
                AccountMeta::new(remote_aggregate, false),
                AccountMeta::new(relay_receipt, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: StrategySpendInstruction::CreditRelayAsset {
                relay_order_id,
                funding_chain_id,
                credit_quantity,
                cost_usdc,
                min_credit_qty: credit_quantity,
                max_credit_qty: credit_quantity,
                nonce: 0,
                deadline: EXPIRES_AT,
            }
            .try_to_vec()
            .unwrap(),
        }
    }

    fn relay_deposit_ix(
        &self,
        relay_order_id: [u8; 32],
        funding_chain_id: u64,
        amount: u64,
        platform_fee_usdc: u64,
        nonce: u64,
    ) -> Instruction {
        let strategy = strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id);
        let vault_authority =
            Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &self.program_id).0;
        let program_authority = Pubkey::find_program_address(
            &[AUTHORITY_SEED, self.owner.pubkey().as_ref()],
            &self.program_id,
        )
        .0;
        let relay_receipt = Pubkey::find_program_address(
            &[
                RELAY_RECEIPT_SEED,
                strategy.as_ref(),
                relay_order_id.as_ref(),
                &[RELAY_ACTION_DEPOSIT],
            ],
            &self.program_id,
        )
        .0;
        let relay_pending_deposit = Pubkey::find_program_address(
            &[
                RELAY_PENDING_DEPOSIT_SEED,
                strategy.as_ref(),
                relay_order_id.as_ref(),
            ],
            &self.program_id,
        )
        .0;
        let strategy_usdc = get_associated_token_address(&vault_authority, &self.usdc_mint);
        let relay_depository_vault =
            Pubkey::find_program_address(&[VAULT_SEED], &self.relay_depository).0;
        let deposit_amount = amount - platform_fee_usdc;
        let mut relay_ix_data = vec![0u8; 8];
        relay_ix_data.extend_from_slice(&deposit_amount.to_le_bytes());
        relay_ix_data.extend_from_slice(relay_order_id.as_ref());

        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new_readonly(self.session.pubkey(), true),
                AccountMeta::new(self.relayer.pubkey(), true),
                AccountMeta::new_readonly(self.owner.pubkey(), false),
                AccountMeta::new_readonly(
                    wallet_pda(&self.program_id, &self.owner.pubkey()),
                    false,
                ),
                AccountMeta::new(strategy, false),
                AccountMeta::new(vault_authority, false),
                AccountMeta::new(
                    get_associated_token_address(&self.owner.pubkey(), &self.usdc_mint),
                    false,
                ),
                AccountMeta::new(strategy_usdc, false),
                AccountMeta::new_readonly(program_authority, false),
                AccountMeta::new(relay_receipt, false),
                AccountMeta::new(relay_pending_deposit, false),
                AccountMeta::new(
                    get_associated_token_address(&self.treasury.pubkey(), &self.usdc_mint),
                    false,
                ),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(self.relay_depository, false),
                AccountMeta::new_readonly(self.usdc_mint, false),
                // deposit_token leg, with the vault swapped into both funding
                // slots Relay quoted for the owner.
                AccountMeta::new_readonly(
                    Pubkey::find_program_address(&[b"state"], &self.relay_depository).0,
                    false,
                ),
                AccountMeta::new(vault_authority, false),
                AccountMeta::new_readonly(self.owner.pubkey(), false),
                AccountMeta::new_readonly(relay_depository_vault, false),
                AccountMeta::new_readonly(self.usdc_mint, false),
                AccountMeta::new(strategy_usdc, false),
                AccountMeta::new(
                    get_associated_token_address(&relay_depository_vault, &self.usdc_mint),
                    false,
                ),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: StrategySpendInstruction::ExecuteRelayDeposit {
                relay_order_id,
                funding_chain_id,
                amount,
                min_dest_amount: deposit_amount,
                locked_cost_usdc: amount - platform_fee_usdc,
                platform_fee_usdc,
                nonce,
                deadline: EXPIRES_AT,
                relay_ix_data,
            }
            .try_to_vec()
            .unwrap(),
        }
    }

    fn credit_usdc_return_ix(
        &self,
        relay_order_id: [u8; 32],
        origin_chain_id: u64,
        gross_return_usdc: u64,
        cost_released_usdc: u64,
        platform_fee_usdc: u64,
        nonce: u64,
    ) -> Instruction {
        let strategy = strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id);
        let vault_authority =
            Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &self.program_id).0;
        let relay_receipt = Pubkey::find_program_address(
            &[
                RELAY_RECEIPT_SEED,
                strategy.as_ref(),
                relay_order_id.as_ref(),
                &[RELAY_ACTION_USDC_RETURN],
            ],
            &self.program_id,
        )
        .0;
        let relay_pending_sell = Pubkey::find_program_address(
            &[
                RELAY_PENDING_SELL_SEED,
                strategy.as_ref(),
                relay_order_id.as_ref(),
            ],
            &self.program_id,
        )
        .0;

        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new_readonly(self.session.pubkey(), true),
                AccountMeta::new(self.relayer.pubkey(), true),
                AccountMeta::new_readonly(self.owner.pubkey(), false),
                AccountMeta::new_readonly(
                    wallet_pda(&self.program_id, &self.owner.pubkey()),
                    false,
                ),
                AccountMeta::new(strategy, false),
                AccountMeta::new_readonly(vault_authority, false),
                AccountMeta::new(
                    get_associated_token_address(&self.owner.pubkey(), &self.usdc_mint),
                    false,
                ),
                AccountMeta::new(
                    get_associated_token_address(&vault_authority, &self.usdc_mint),
                    false,
                ),
                AccountMeta::new(
                    get_associated_token_address(&self.treasury.pubkey(), &self.usdc_mint),
                    false,
                ),
                AccountMeta::new(relay_receipt, false),
                AccountMeta::new(relay_pending_sell, false),
                AccountMeta::new_readonly(self.usdc_mint, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: StrategySpendInstruction::CreditUsdcReturn {
                relay_order_id,
                funding_chain_id: SOLANA_RELAY_CHAIN_ID,
                origin_chain_id,
                gross_return_usdc,
                quantity_released: 1_000_000,
                cost_released_usdc,
                platform_fee_usdc,
                nonce,
                deadline: EXPIRES_AT,
            }
            .try_to_vec()
            .unwrap(),
        }
    }

    fn remote_relay_sell_ix(
        &self,
        relay_order_id: [u8; 32],
        funding_chain_id: u64,
        sell_quantity: u64,
        return_usdc: u64,
        nonce: u64,
    ) -> Instruction {
        let strategy = strategy_pda(&self.program_id, &self.owner.pubkey(), &self.strategy_id);
        let vault_authority =
            Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &self.program_id).0;
        let wsol_vault =
            get_associated_token_address(&vault_authority, &spl_token::native_mint::id());
        let remote_asset = Pubkey::find_program_address(
            &[
                REMOTE_ASSET_SEED,
                strategy.as_ref(),
                spl_token::native_mint::id().as_ref(),
                &funding_chain_id.to_le_bytes(),
            ],
            &self.program_id,
        )
        .0;
        let remote_aggregate = Pubkey::find_program_address(
            &[
                REMOTE_AGGREGATE_SEED,
                strategy.as_ref(),
                spl_token::native_mint::id().as_ref(),
            ],
            &self.program_id,
        )
        .0;
        let relay_receipt = Pubkey::find_program_address(
            &[
                RELAY_RECEIPT_SEED,
                strategy.as_ref(),
                relay_order_id.as_ref(),
                &[RELAY_ACTION_REMOTE_SELL],
            ],
            &self.program_id,
        )
        .0;
        let relay_pending_sell = Pubkey::find_program_address(
            &[
                RELAY_PENDING_SELL_SEED,
                strategy.as_ref(),
                relay_order_id.as_ref(),
            ],
            &self.program_id,
        )
        .0;
        let forwarder_usdc = get_associated_token_address(&self.forwarder, &self.usdc_mint);
        let relay_vault_usdc = get_associated_token_address(
            &strategy_spend::processor::RELAY_FORWARDER_PROGRAM_ID,
            &self.usdc_mint,
        );

        let mut swap_ix_data = vec![7u8];
        swap_ix_data.extend_from_slice(&sell_quantity.to_le_bytes());
        swap_ix_data.extend_from_slice(&return_usdc.to_le_bytes());

        Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new_readonly(self.session.pubkey(), true),
                AccountMeta::new(self.relayer.pubkey(), true),
                AccountMeta::new_readonly(self.owner.pubkey(), false),
                AccountMeta::new_readonly(
                    wallet_pda(&self.program_id, &self.owner.pubkey()),
                    false,
                ),
                AccountMeta::new(strategy, false),
                AccountMeta::new(vault_authority, false),
                AccountMeta::new(wsol_vault, false),
                AccountMeta::new_readonly(spl_token::native_mint::id(), false),
                AccountMeta::new(remote_asset, false),
                AccountMeta::new(remote_aggregate, false),
                AccountMeta::new(relay_receipt, false),
                AccountMeta::new(relay_pending_sell, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(self.jupiter_program, false),
                AccountMeta::new_readonly(
                    strategy_spend::processor::RELAY_FORWARDER_PROGRAM_ID,
                    false,
                ),
                // Swap leg: vault WSOL out, USDC into the Relay forwarder.
                AccountMeta::new_readonly(vault_authority, false),
                AccountMeta::new(wsol_vault, false),
                AccountMeta::new(forwarder_usdc, false),
                AccountMeta::new(spl_token::native_mint::id(), false),
                AccountMeta::new(self.usdc_mint, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new(self.native_output_wsol, false),
                // forward_token leg.
                AccountMeta::new_readonly(vault_authority, false),
                AccountMeta::new_readonly(self.forwarder, false),
                AccountMeta::new(forwarder_usdc, false),
                AccountMeta::new(relay_vault_usdc, false),
                AccountMeta::new_readonly(self.usdc_mint, false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
            data: StrategySpendInstruction::ExecuteRemoteRelaySell {
                relay_order_id,
                funding_chain_id,
                sell_quantity,
                min_return_usdc: return_usdc,
                nonce,
                deadline: EXPIRES_AT,
                swap_ix_data,
                relay_ix_data: relay_order_id.to_vec(),
            }
            .try_to_vec()
            .unwrap(),
        }
    }
}

async fn send(
    banks_client: &mut solana_program_test::BanksClient,
    payer: &Keypair,
    signers: &[&Keypair],
    ix: Instruction,
) -> Result<(), BanksClientError> {
    let blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), signers, blockhash);
    banks_client.process_transaction(tx).await
}

async fn bootstrap(
    h: &TestHarness,
    banks_client: &mut solana_program_test::BanksClient,
    payer: &Keypair,
) {
    send(banks_client, payer, &[payer, &h.owner], h.init_wallet_ix())
        .await
        .unwrap();
    send(
        banks_client,
        payer,
        &[payer, &h.owner],
        h.init_strategy_ix(),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn init_wallet_and_strategy() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;

    let wallet = wallet_pda(&h.program_id, &h.owner.pubkey());
    let wallet_account = banks_client.get_account(wallet).await.unwrap().unwrap();
    let config = WalletConfig::try_from_slice(&wallet_account.data).unwrap();
    assert_eq!(config.owner, h.owner.pubkey());
    assert_eq!(config.usdc_mint, h.usdc_mint);

    let state = h.read_strategy(&mut banks_client).await;
    assert_eq!(state.limit_usdc, LIMIT_USDC);
    assert_eq!(state.capacity_usdc, LIMIT_USDC);
    assert_eq!(state.deployed_usdc, 0);
}

#[tokio::test]
async fn init_wallet_migrates_legacy_v1_account() {
    let (h, mut banks_client, payer) = TestHarness::start_with_legacy_v1_wallet().await;
    send(
        &mut banks_client,
        &payer,
        &[&payer, &h.owner],
        h.init_wallet_ix(),
    )
    .await
    .unwrap();
    send(
        &mut banks_client,
        &payer,
        &[&payer, &h.owner],
        h.init_strategy_ix(),
    )
    .await
    .unwrap();

    let wallet = wallet_pda(&h.program_id, &h.owner.pubkey());
    let wallet_account = banks_client.get_account(wallet).await.unwrap().unwrap();
    let config = WalletConfig::try_from_slice(&wallet_account.data).unwrap();
    assert_eq!(wallet_account.data.len(), WalletConfig::LEN);
    assert_eq!(config.owner, h.owner.pubkey());
    assert_eq!(config.usdc_mint, h.usdc_mint);
    assert_eq!(config.jupiter_program, h.jupiter_program);
    assert_eq!(config.platform_relayer, h.relayer.pubkey());
}

#[tokio::test]
async fn init_strategy_is_create_only() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    let mut duplicate_init = h.init_strategy_ix();
    duplicate_init.data = StrategySpendInstruction::InitStrategy {
        strategy_id: h.strategy_id,
        session: h.session.pubkey(),
        limit_usdc: LIMIT_USDC - 1,
        expires_at: EXPIRES_AT,
    }
    .try_to_vec()
    .unwrap();
    assert!(send(
        &mut banks_client,
        &payer,
        &[&payer, &h.owner],
        duplicate_init
    )
    .await
    .is_err());
}

#[tokio::test]
async fn set_limit_updates_limit_and_expiry_not_deployed() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;

    send(
        &mut banks_client,
        &payer,
        &[&payer, &h.owner],
        h.set_limit_ix(500_000_000, EXPIRES_AT + 3600),
    )
    .await
    .unwrap();

    let state = h.read_strategy(&mut banks_client).await;
    assert_eq!(state.limit_usdc, 500_000_000);
    assert_eq!(state.deployed_usdc, 0);
    assert_eq!(state.expires_at, EXPIRES_AT + 3600);
}

#[tokio::test]
async fn set_limit_preserves_losses_and_does_not_unrevoke() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    send(
        &mut banks_client,
        &payer,
        &[&payer, &h.session],
        h.revoke_ix(&h.session),
    )
    .await
    .unwrap();
    send(
        &mut banks_client,
        &payer,
        &[&payer, &h.owner],
        h.set_limit_ix(LIMIT_USDC + 100_000_000, EXPIRES_AT + 3600),
    )
    .await
    .unwrap();

    let state = h.read_strategy(&mut banks_client).await;
    assert!(state.revoked);
    assert_eq!(state.capacity_usdc, LIMIT_USDC + 100_000_000);
}

#[tokio::test]
async fn set_limit_rejects_expired_grants() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    assert!(send(
        &mut banks_client,
        &payer,
        &[&payer, &h.owner],
        h.set_limit_ix(LIMIT_USDC, 1),
    )
    .await
    .is_err());
}

#[tokio::test]
async fn execute_buy_sell_and_recovery_use_measured_deltas() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    let strategy = strategy_pda(&h.program_id, &h.owner.pubkey(), &h.strategy_id);
    let vault_authority =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &h.program_id).0;
    let token_vault = get_associated_token_address(&vault_authority, &h.token_mint);
    assert!(banks_client
        .get_account(token_vault)
        .await
        .unwrap()
        .is_none());

    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.execute_swap_with_fees_ix(
            true,
            200_000_000,
            90_000_000,
            0,
            GasMode::None,
            0,
            0,
            200_000_000,
            100_000_000,
            0,
            0,
        ),
    )
    .await
    .unwrap();
    assert!(banks_client
        .get_account(token_vault)
        .await
        .unwrap()
        .is_some());
    let asset_key = Pubkey::find_program_address(
        &[ASSET_SEED, strategy.as_ref(), h.token_mint.as_ref()],
        &h.program_id,
    )
    .0;
    let asset_account = banks_client.get_account(asset_key).await.unwrap().unwrap();
    let asset = StrategyAsset::try_from_slice(&asset_account.data).unwrap();
    assert_eq!(asset.quantity, 100_000_000);
    assert_eq!(asset.cost_usdc, 200_000_000);

    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.execute_swap_with_fees_ix(
            false,
            75_000_000,
            50_000_000,
            0,
            GasMode::None,
            0,
            0,
            50_000_000,
            80_000_000,
            0,
            0,
        ),
    )
    .await
    .unwrap();
    let state = h.read_strategy(&mut banks_client).await;
    assert_eq!(state.deployed_usdc, 100_000_000);
    assert_eq!(state.capacity_usdc, 980_000_000);
    send(
        &mut banks_client,
        &payer,
        &[&payer, &h.owner],
        h.set_limit_ix(LIMIT_USDC + 100_000_000, EXPIRES_AT + 3600),
    )
    .await
    .unwrap();
    assert_eq!(
        h.read_strategy(&mut banks_client).await.capacity_usdc,
        1_080_000_000
    );

    send(
        &mut banks_client,
        &payer,
        &[&payer, &h.owner],
        h.withdraw_ix(10_000_000),
    )
    .await
    .unwrap();
    let state = h.read_strategy(&mut banks_client).await;
    assert_eq!(state.deployed_usdc, 80_000_000);
}

#[tokio::test]
async fn revoke_marks_strategy_revoked() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;

    send(
        &mut banks_client,
        &payer,
        &[&payer, &h.session],
        h.revoke_ix(&h.session),
    )
    .await
    .unwrap();

    assert!(h.read_strategy(&mut banks_client).await.revoked);
    assert_eq!(h.read_strategy(&mut banks_client).await.nonce, 1);
}

#[tokio::test]
async fn rotate_session_bumps_nonce() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    send(
        &mut banks_client,
        &payer,
        &[&payer, &h.session],
        h.revoke_ix(&h.session),
    )
    .await
    .unwrap();

    let new_session = Keypair::new();
    send(
        &mut banks_client,
        &payer,
        &[&payer, &h.owner],
        h.rotate_session_ix(&new_session.pubkey()),
    )
    .await
    .unwrap();

    let state = h.read_strategy(&mut banks_client).await;
    assert_eq!(state.session, new_session.pubkey());
    assert_eq!(state.nonce, 2);
    assert!(!state.revoked);
}

#[tokio::test]
async fn close_strategy_when_flat() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;

    send(
        &mut banks_client,
        &payer,
        &[&payer, &h.owner],
        h.close_strategy_ix(),
    )
    .await
    .unwrap();

    let strategy = strategy_pda(&h.program_id, &h.owner.pubkey(), &h.strategy_id);
    assert!(banks_client.get_account(strategy).await.unwrap().is_none());
}

#[tokio::test]
async fn execute_swap_with_fees_buy_charges_treasury_and_reduces_capacity() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    let relayer_before = banks_client
        .get_account(h.relayer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.execute_swap_with_fees_ix(
            true,
            200_000_000,
            90_000_000,
            1_000_000,
            GasMode::Separate,
            500_000,
            10_000_000,
            200_000_000,
            100_000_000,
            500_000,
            10_000_000,
        ),
    )
    .await
    .unwrap();
    let relayer_after = banks_client
        .get_account(h.relayer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert!(relayer_after >= relayer_before + 10_000_000);

    let state = h.read_strategy(&mut banks_client).await;
    assert_eq!(state.deployed_usdc, 200_000_000);
    assert_eq!(state.capacity_usdc, LIMIT_USDC - 1_000_000 - 500_000);

    let treasury_usdc = get_associated_token_address(&h.treasury.pubkey(), &h.usdc_mint);
    let treasury_account = banks_client
        .get_account(treasury_usdc)
        .await
        .unwrap()
        .unwrap();
    let treasury_token = TokenAccount::unpack(&treasury_account.data).unwrap();
    assert_eq!(treasury_token.amount, 1_000_000);
}

#[tokio::test]
async fn execute_swap_with_fees_rejects_gas_below_minimum_atomically() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    let owner_usdc = get_associated_token_address(&h.owner.pubkey(), &h.usdc_mint);
    let owner_before = banks_client.get_account(owner_usdc).await.unwrap().unwrap();
    let owner_before = TokenAccount::unpack(&owner_before.data).unwrap().amount;

    assert!(send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.execute_swap_with_fees_ix(
            true,
            200_000_000,
            90_000_000,
            0,
            GasMode::Separate,
            500_000,
            10_000_001,
            200_000_000,
            100_000_000,
            500_000,
            10_000_000,
        ),
    )
    .await
    .is_err());

    let owner_after = banks_client.get_account(owner_usdc).await.unwrap().unwrap();
    assert_eq!(
        TokenAccount::unpack(&owner_after.data).unwrap().amount,
        owner_before
    );
}

#[tokio::test]
async fn execute_swap_with_fees_buy_rejects_insufficient_deployable_for_fees() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;

    assert!(send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.execute_swap_with_fees_ix(
            true,
            LIMIT_USDC,
            90_000_000,
            1_000_000,
            GasMode::Separate,
            500_000,
            10_000_000,
            LIMIT_USDC,
            100_000_000,
            500_000,
            10_000_000,
        ),
    )
    .await
    .is_err());
}

#[tokio::test]
async fn execute_swap_with_fees_sell_applies_fees_after_swap() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;

    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.execute_swap_with_fees_ix(
            true,
            200_000_000,
            90_000_000,
            0,
            GasMode::None,
            0,
            0,
            200_000_000,
            100_000_000,
            0,
            0,
        ),
    )
    .await
    .unwrap();

    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.execute_swap_with_fees_ix(
            false,
            75_000_000,
            50_000_000,
            1_000_000,
            GasMode::Separate,
            500_000,
            10_000_000,
            50_000_000,
            80_000_000,
            500_000,
            10_000_000,
        ),
    )
    .await
    .unwrap();

    let state = h.read_strategy(&mut banks_client).await;
    assert_eq!(state.deployed_usdc, 100_000_000);
    assert_eq!(state.capacity_usdc, 980_000_000 - 1_000_000 - 500_000);
}

#[tokio::test]
async fn execute_swap_v2_separate_logs_verified_native_credit() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    let instruction =
        h.execute_swap_with_fees_v2_separate_ix(h.relayer.pubkey(), 10_000_000, 9_000_000);
    let blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&h.relayer.pubkey()),
        &[&h.relayer, &h.session],
        blockhash,
    );
    let result = banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();
    result.result.unwrap();
    let expected = format!(
        "Program log: POCKLESS_GAS_CREDIT_V1:separate:{}:{}:10000000",
        strategy_pda(&h.program_id, &h.owner.pubkey(), &h.strategy_id),
        h.relayer.pubkey()
    );
    assert!(result.metadata.unwrap().log_messages.contains(&expected));
}

#[tokio::test]
async fn execute_swap_v2_rejects_gas_recipient_mismatch() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    assert!(send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.execute_swap_with_fees_v2_separate_ix(Pubkey::new_unique(), 10_000_000, 9_000_000),
    )
    .await
    .is_err());
}

#[tokio::test]
async fn execute_swap_v2_credit_only_uses_existing_credit_without_new_debit() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.execute_swap_with_fees_v2_credit_only_ix(),
    )
    .await
    .unwrap();

    let state = h.read_strategy(&mut banks_client).await;
    assert_eq!(state.deployed_usdc, 200_000_000);
    assert_eq!(state.capacity_usdc, LIMIT_USDC);
}

#[tokio::test]
async fn execute_swap_v2_credit_only_buys_native_sol() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.execute_wsol_credit_only_ix(),
    )
    .await
    .unwrap();

    let state = h.read_strategy(&mut banks_client).await;
    assert_eq!(state.deployed_usdc, 200_000_000);
}

#[tokio::test]
async fn execute_swap_v2_native_output_splits_gas_and_records_net_inventory() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    let instruction = h.execute_native_output_ix(h.relayer.pubkey(), 10_000_000, 100_000_000);
    let blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&h.relayer.pubkey()),
        &[&h.relayer, &h.session],
        blockhash,
    );
    let result = banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();
    result.result.unwrap();

    let strategy = strategy_pda(&h.program_id, &h.owner.pubkey(), &h.strategy_id);
    let asset_key = Pubkey::find_program_address(
        &[
            ASSET_SEED,
            strategy.as_ref(),
            spl_token::native_mint::id().as_ref(),
        ],
        &h.program_id,
    )
    .0;
    let asset_account = banks_client.get_account(asset_key).await.unwrap().unwrap();
    let asset = StrategyAsset::try_from_slice(&asset_account.data).unwrap();
    assert_eq!(asset.quantity, 90_000_000);
    assert_eq!(asset.cost_usdc, 200_000_000);
    let state = h.read_strategy(&mut banks_client).await;
    assert_eq!(state.deployed_usdc, 200_000_000);
    assert_eq!(state.capacity_usdc, LIMIT_USDC - 500_000);
    assert!(banks_client
        .get_account(h.native_output_wsol)
        .await
        .unwrap()
        .is_none());
    let expected = format!(
        "Program log: POCKLESS_GAS_CREDIT_V1:native_output:{}:{}:10000000",
        strategy,
        h.relayer.pubkey()
    );
    assert!(result.metadata.unwrap().log_messages.contains(&expected));
}

#[tokio::test]
async fn execute_swap_v2_native_output_rejects_invalid_modes_and_split() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;

    let mut sell = h.execute_native_output_ix(h.relayer.pubkey(), 10_000_000, 100_000_000);
    sell.data[1] = 0;
    assert!(send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        sell,
    )
    .await
    .is_err());

    let mut non_wsol = h.execute_native_output_ix(h.relayer.pubkey(), 10_000_000, 100_000_000);
    non_wsol.accounts[11].pubkey = h.token_mint;
    assert!(send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        non_wsol,
    )
    .await
    .is_err());

    let over_split = h.execute_native_output_ix(h.relayer.pubkey(), 100_000_001, 100_000_000);
    assert!(send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        over_split,
    )
    .await
    .is_err());

    let mut invalid_none = h.execute_native_output_ix(h.relayer.pubkey(), 10_000_000, 100_000_000);
    invalid_none.data[26] = 0;
    assert!(send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        invalid_none,
    )
    .await
    .is_err());
}

#[tokio::test]
async fn pro_rata_cost_matches_on_chain_math() {
    let total_cost = 900u64;
    let total_qty = 300u64;
    let sold_qty = 100u64;
    let expected = total_cost * sold_qty / total_qty;
    assert_eq!(expected, 300);
}

#[tokio::test]
async fn credit_relay_wraps_native_sol_into_wsol_surplus() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    let strategy = strategy_pda(&h.program_id, &h.owner.pubkey(), &h.strategy_id);
    let vault_authority =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &h.program_id).0;
    let wsol_vault = get_associated_token_address(&vault_authority, &spl_token::native_mint::id());
    let credit_quantity = 29_850_255u64;
    let leftover = 20_149_745u64;
    send(
        &mut banks_client,
        &payer,
        &[&payer],
        system_instruction::transfer(
            &payer.pubkey(),
            &vault_authority,
            credit_quantity + leftover,
        ),
    )
    .await
    .unwrap();

    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.credit_relay_asset_ix([7u8; 32], 8453, credit_quantity, 3_000_000),
    )
    .await
    .unwrap();

    let wsol = TokenAccount::unpack(
        &banks_client
            .get_account(wsol_vault)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(wsol.amount, credit_quantity);
    assert_eq!(
        banks_client
            .get_account(vault_authority)
            .await
            .unwrap()
            .unwrap()
            .lamports,
        leftover
    );
    let remote_asset = Pubkey::find_program_address(
        &[
            REMOTE_ASSET_SEED,
            strategy.as_ref(),
            spl_token::native_mint::id().as_ref(),
            &8453u64.to_le_bytes(),
        ],
        &h.program_id,
    )
    .0;
    let remote = RemoteStrategyAsset::try_from_slice(
        &banks_client
            .get_account(remote_asset)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(remote.quantity, credit_quantity);
    assert_eq!(remote.cost_usdc, 3_000_000);
    let remote_aggregate = Pubkey::find_program_address(
        &[
            REMOTE_AGGREGATE_SEED,
            strategy.as_ref(),
            spl_token::native_mint::id().as_ref(),
        ],
        &h.program_id,
    )
    .0;
    let aggregate = RemoteMintAggregate::try_from_slice(
        &banks_client
            .get_account(remote_aggregate)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(aggregate.total_accounted, credit_quantity);
}

#[tokio::test]
async fn relay_deposit_sends_vault_usdc_through_the_depository() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    let strategy = strategy_pda(&h.program_id, &h.owner.pubkey(), &h.strategy_id);
    let vault_authority =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &h.program_id).0;
    let amount = 25_000_000u64;
    let platform_fee = 250_000u64;

    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.relay_deposit_ix([21u8; 32], 8453, amount, platform_fee, 0),
    )
    .await
    .unwrap();

    let relay_vault = Pubkey::find_program_address(&[VAULT_SEED], &h.relay_depository).0;
    assert_eq!(
        token_balance(
            &mut banks_client,
            get_associated_token_address(&relay_vault, &h.usdc_mint)
        )
        .await,
        amount - platform_fee
    );
    assert_eq!(
        token_balance(
            &mut banks_client,
            get_associated_token_address(&vault_authority, &h.usdc_mint)
        )
        .await,
        0
    );
    assert_eq!(
        token_balance(
            &mut banks_client,
            get_associated_token_address(&h.treasury.pubkey(), &h.usdc_mint)
        )
        .await,
        platform_fee
    );
    assert_eq!(
        token_balance(
            &mut banks_client,
            get_associated_token_address(&h.owner.pubkey(), &h.usdc_mint)
        )
        .await,
        LIMIT_USDC - amount
    );
}

/// A position sold on an EVM venue records its pending sell on that chain, so
/// the Solana funding-chain credit has no local record to consume.
#[tokio::test]
async fn credit_usdc_return_accepts_a_remote_origin_sell() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    let strategy = strategy_pda(&h.program_id, &h.owner.pubkey(), &h.strategy_id);
    let vault_authority =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &h.program_id).0;
    let owner_usdc = get_associated_token_address(&h.owner.pubkey(), &h.usdc_mint);
    let vault_usdc = get_associated_token_address(&vault_authority, &h.usdc_mint);

    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.relay_deposit_ix([31u8; 32], 8453, 25_000_000, 250_000, 0),
    )
    .await
    .unwrap();

    // Relay delivers the sell proceeds to the vault, which is what the credit
    // pays the fee and the owner out of.
    send(
        &mut banks_client,
        &payer,
        &[&payer, &h.owner],
        token_instruction::transfer_checked(
            &spl_token::id(),
            &owner_usdc,
            &h.usdc_mint,
            &vault_usdc,
            &h.owner.pubkey(),
            &[],
            26_000_000,
            6,
        )
        .unwrap(),
    )
    .await
    .unwrap();

    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.credit_usdc_return_ix([32u8; 32], 8453, 26_000_000, 24_750_000, 260_000, 1),
    )
    .await
    .unwrap();

    assert_eq!(token_balance(&mut banks_client, vault_usdc).await, 0);
    assert_eq!(
        token_balance(&mut banks_client, owner_usdc).await,
        LIMIT_USDC - 25_000_000 - 26_000_000 + 26_000_000 - 260_000
    );
    assert_eq!(
        token_balance(
            &mut banks_client,
            get_associated_token_address(&h.treasury.pubkey(), &h.usdc_mint)
        )
        .await,
        250_000 + 260_000
    );
    let state = h.read_strategy(&mut banks_client).await;
    assert_eq!(state.deployed_usdc, 0);
}

async fn token_balance(
    banks_client: &mut solana_program_test::BanksClient,
    account: Pubkey,
) -> u64 {
    TokenAccount::unpack(
        &banks_client
            .get_account(account)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap()
    .amount
}

#[tokio::test]
async fn remote_relay_sell_swaps_inventory_into_the_relay_forwarder() {
    let (h, mut banks_client, payer) = TestHarness::start().await;
    bootstrap(&h, &mut banks_client, &payer).await;
    let strategy = strategy_pda(&h.program_id, &h.owner.pubkey(), &h.strategy_id);
    let vault_authority =
        Pubkey::find_program_address(&[VAULT_SEED, strategy.as_ref()], &h.program_id).0;
    let wsol_vault = get_associated_token_address(&vault_authority, &spl_token::native_mint::id());
    let sell_quantity = 29_850_255u64;
    let return_usdc = 2_922_821u64;
    send(
        &mut banks_client,
        &payer,
        &[&payer],
        system_instruction::transfer(&payer.pubkey(), &vault_authority, sell_quantity),
    )
    .await
    .unwrap();
    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.credit_relay_asset_ix([11u8; 32], 8453, sell_quantity, 3_039_707),
    )
    .await
    .unwrap();

    send(
        &mut banks_client,
        &h.relayer,
        &[&h.relayer, &h.session],
        h.remote_relay_sell_ix([12u8; 32], 8453, sell_quantity, return_usdc, 1),
    )
    .await
    .unwrap();

    let vault = TokenAccount::unpack(
        &banks_client
            .get_account(wsol_vault)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(vault.amount, 0);
    let relay_vault_usdc = TokenAccount::unpack(
        &banks_client
            .get_account(get_associated_token_address(
                &strategy_spend::processor::RELAY_FORWARDER_PROGRAM_ID,
                &h.usdc_mint,
            ))
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(relay_vault_usdc.amount, return_usdc);
    let forwarder_usdc = TokenAccount::unpack(
        &banks_client
            .get_account(get_associated_token_address(&h.forwarder, &h.usdc_mint))
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(forwarder_usdc.amount, 0);

    let remote_asset = Pubkey::find_program_address(
        &[
            REMOTE_ASSET_SEED,
            strategy.as_ref(),
            spl_token::native_mint::id().as_ref(),
            &8453u64.to_le_bytes(),
        ],
        &h.program_id,
    )
    .0;
    let remote = RemoteStrategyAsset::try_from_slice(
        &banks_client
            .get_account(remote_asset)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(remote.quantity, 0);
    assert_eq!(remote.cost_usdc, 0);
}
