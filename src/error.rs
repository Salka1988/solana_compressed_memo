use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum ExtendedSPLMemoError {
    #[error("Invalid Instruction")]
    InvalidInstruction,
    #[error("Memo too long")]
    MemoTooLong,
    #[error("Account data too small")]
    AccountDataTooSmall,
    #[error("Hashing error")]
    HashingError,
}

impl From<ExtendedSPLMemoError> for ProgramError {
    fn from(e: ExtendedSPLMemoError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
