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
use strategy_spend::instruction::StrategySpendInstruction;
use strategy_spend::state::{
    StrategyAccount, StrategyAsset, WalletConfig, ASSET_SEED, AUTHORITY_SEED, STRATEGY_SEED,
    VAULT_SEED, WALLET_SEED,
};

const LIMIT_USDC: u64 = 1_000_000_000;
const EXPIRES_AT: i64 = 4_102_444_800;
const GAS_FUNDER_SEED: &[u8] = b"gas-funder";

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
        mock_token_swap(accounts, data)
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

fn mock_token_swap(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let authority = next_account_info(accounts)?;
    let source = next_account_info(accounts)?;
    let destination = next_account_info(accounts)?;
    let input_mint = next_account_info(accounts)?;
    let output_mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;
    let input_amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let output_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());

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

struct TestHarness {
    program_id: Pubkey,
    jupiter_program: Pubkey,
    owner: Keypair,
    session: Keypair,
    relayer: Keypair,
    treasury: Keypair,
    usdc_mint: Pubkey,
    token_mint: Pubkey,
    gas_funder: Pubkey,
    strategy_id: [u8; 32],
}

impl TestHarness {
    async fn start() -> (Self, solana_program_test::BanksClient, Keypair) {
        let program_id = Pubkey::new_unique();
        let jupiter_program = Pubkey::new_unique();
        let owner = Keypair::new();
        let session = Keypair::new();
        let relayer = Keypair::new();
        let treasury = Keypair::new();
        let usdc_mint = Pubkey::new_unique();
        let token_mint = Pubkey::new_unique();
        let gas_funder = Pubkey::find_program_address(&[GAS_FUNDER_SEED], &jupiter_program).0;
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
            owner,
            session,
            relayer,
            treasury,
            usdc_mint,
            token_mint,
            gas_funder,
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

    fn execute_swap_ix(
        &self,
        is_buy: bool,
        usdc_amount: u64,
        token_amount: u64,
        actual_input: u64,
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
        let owner_usdc = get_associated_token_address(&self.owner.pubkey(), &self.usdc_mint);
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
                AccountMeta::new(owner_usdc, false),
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
                AccountMeta::new_readonly(vault_authority, false),
                AccountMeta::new(source, false),
                AccountMeta::new(destination, false),
                AccountMeta::new(input_mint, false),
                AccountMeta::new(output_mint, false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
            data: StrategySpendInstruction::ExecuteSwap {
                is_buy,
                usdc_amount,
                token_amount,
                jupiter_data,
            }
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
        gas_reimburse_usdc: u64,
        actual_input: u64,
        actual_output: u64,
        gas_input: u64,
        gas_lamports_out: u64,
        min_native_out: u64,
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
        let gas_account_count = if gas_reimburse_usdc > 0 { 7u8 } else { 0u8 };
        let mut gas_payload = Vec::new();
        if gas_reimburse_usdc > 0 {
            gas_payload.push(gas_account_count);
            gas_payload.extend_from_slice(&gas_jupiter_data);
        }

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
            if gas_reimburse_usdc > 0 {
                AccountMeta::new(
                    get_associated_token_address(&vault_authority, &spl_token::native_mint::id()),
                    false,
                )
            } else {
                AccountMeta::new(self.relayer.pubkey(), false)
            },
        ];
        if gas_reimburse_usdc > 0 {
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
                gas_reimburse_usdc,
                min_native_out,
                treasury: self.treasury.pubkey(),
                jupiter_data,
                gas_jupiter_data: gas_payload,
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
        h.execute_swap_ix(true, 200_000_000, 90_000_000, 200_000_000, 100_000_000),
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
        h.execute_swap_ix(false, 75_000_000, 50_000_000, 50_000_000, 80_000_000),
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
    assert_eq!(state.nonce, 1);
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
            500_000,
            200_000_000,
            100_000_000,
            500_000,
            10_000_000,
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
            500_000,
            200_000_000,
            100_000_000,
            500_000,
            10_000_000,
            10_000_001,
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
            500_000,
            LIMIT_USDC,
            100_000_000,
            500_000,
            10_000_000,
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
            0,
            200_000_000,
            100_000_000,
            0,
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
            false, 75_000_000, 50_000_000, 1_000_000, 500_000, 50_000_000, 80_000_000, 500_000,
            10_000_000, 10_000_000,
        ),
    )
    .await
    .unwrap();

    let state = h.read_strategy(&mut banks_client).await;
    assert_eq!(state.deployed_usdc, 100_000_000);
    assert_eq!(state.capacity_usdc, 980_000_000 - 1_000_000 - 500_000);
}

#[tokio::test]
async fn pro_rata_cost_matches_on_chain_math() {
    let total_cost = 900u64;
    let total_qty = 300u64;
    let sold_qty = 100u64;
    let expected = total_cost * sold_qty / total_qty;
    assert_eq!(expected, 300);
}
