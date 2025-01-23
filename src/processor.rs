use borsh::BorshDeserialize;
use light_sdk_macros::LightHasher;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    error::ExtendedSPLMemoError,
    instruction::ExtendedSPLMemoInstruction,
};

use light_hasher::{DataHasher, Poseidon};

/// A simple struct that holds the memo. We'll derive LightHasher for it.
#[derive(LightHasher)]
pub struct CompressedMemo {
    #[truncate]
    pub memo: String,
}

pub const MAX_MEMO_LEN: usize = 128; // arbitrary for demo

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = ExtendedSPLMemoInstruction::try_from_slice(instruction_data)
        .map_err(|_| ExtendedSPLMemoError::InvalidInstruction)?;

    match instruction {
        ExtendedSPLMemoInstruction::OriginalMemo { memo } => {
            process_original_memo(accounts, &memo)
        }
        ExtendedSPLMemoInstruction::CreateCompressedMemo { memo } => {
            process_create_compressed_memo(program_id, accounts, &memo)
        }
    }
}

fn process_original_memo(
    accounts: &[AccountInfo],
    memo: &str,
) -> ProgramResult {
    if memo.len() > MAX_MEMO_LEN {
        return Err(ExtendedSPLMemoError::MemoTooLong.into());
    }
    // Just log it, as SPL Memo does
    msg!("Memo: {}", memo);
    Ok(())
}

fn process_create_compressed_memo(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    memo: &str,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer_info = next_account_info(account_info_iter)?;
    let new_account_info = next_account_info(account_info_iter)?;

    if !payer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if memo.len() > MAX_MEMO_LEN {
        return Err(ExtendedSPLMemoError::MemoTooLong.into());
    }

    // Construct our CompressedMemo struct
    let compressed_memo_struct = CompressedMemo {
        memo: memo.to_string(),
    };

    // Derive-macro provided .hash() => typically returns a [u8; 32] and use Poseidon hasher
    let hashed_data = compressed_memo_struct.hash::<Poseidon>().map_err(|_| ExtendedSPLMemoError::HashingError)?;

    // Create the account if it's not already
    if new_account_info.data_is_empty() {
        let rent = Rent::get()?;
        let required_lamports = rent.minimum_balance(hashed_data.len());

        let create_ix = system_instruction::create_account(
            payer_info.key,
            new_account_info.key,
            required_lamports,
            hashed_data.len() as u64,
            program_id,
        );
        invoke(
            &create_ix,
            &[
                payer_info.clone(),
                new_account_info.clone(),
            ],
        )?;
    }

    // Write the hashed data to the new account
    let account_data_slice = &mut new_account_info.try_borrow_mut_data()?[..hashed_data.len()];
    account_data_slice.copy_from_slice(&hashed_data);

    msg!("Compressed memo created and stored!");
    Ok(())
}
