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
    #[error("Platform relayer mismatch")]
    RelayerMismatch,
    #[error("Relay intent nonce mismatch")]
    NonceMismatch,
    #[error("Relay order already consumed")]
    RelayOrderConsumed,
    #[error("Insufficient vault surplus for relay credit")]
    InsufficientVaultSurplus,
    #[error("Relay credit below minimum")]
    RelayCreditBelowMinimum,
    #[error("Remote inventory insufficient")]
    InsufficientRemoteAsset,
    #[error("Relay depository CPI validation failed")]
    RelayCpiFailed,
    #[error("Relay pending record missing")]
    PendingRecordMissing,
}

impl From<StrategySpendError> for ProgramError {
    fn from(error: StrategySpendError) -> Self {
        ProgramError::Custom(error as u32)
    }
}
