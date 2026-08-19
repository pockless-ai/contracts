use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum StrategySpendError {
    #[error("Invalid instruction")]
    InvalidInstruction,
    #[error("Invalid account")]
    InvalidAccount,
    #[error("Missing required signature")]
    MissingSignature,
    #[error("Account already initialized")]
    AlreadyInitialized,
    #[error("Strategy is revoked")]
    Revoked,
    #[error("Strategy has expired")]
    Expired,
    #[error("Spend exceeds deployable capacity")]
    CapacityExceeded,
    #[error("Session mismatch")]
    SessionMismatch,
    #[error("Owner mismatch")]
    OwnerMismatch,
    #[error("Mint mismatch")]
    MintMismatch,
    #[error("Program mismatch")]
    ProgramMismatch,
    #[error("Insufficient asset balance")]
    InsufficientAsset,
    #[error("Strategy has open deployment")]
    OpenDeployment,
    #[error("Arithmetic overflow")]
    Overflow,
    #[error("Jupiter CPI failed")]
    JupiterCpiFailed,
}

impl From<StrategySpendError> for ProgramError {
    fn from(error: StrategySpendError) -> Self {
        ProgramError::Custom(error as u32)
    }
}
