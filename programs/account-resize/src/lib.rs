//! An ABIv2 program that demonstrates how to resize and account.
//!
//! The program expects an empty account to resize. The final size of the
//! account is determined by the instruction data.

#![no_std]

use abiv2::{
    account::Account, context::InstructionContext, entrypoint, error::ProgramError, ProgramResult,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _context: &InstructionContext,
    accounts: &[Account],
    instruction_data: &[u8],
) -> ProgramResult {
    let &[mut account, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let new_len = if instruction_data.len() == 8 {
        // SAFETY: The instruction data is guaranteed to be 8 bytes long.
        u64::from_le_bytes(unsafe { *(instruction_data.as_ptr() as *const [u8; 8]) })
    } else {
        return Err(ProgramError::InvalidInstructionData);
    };

    if account.data_len() != 0 {
        return Err(ProgramError::InvalidAccountData);
    }

    account.resize(new_len as usize)?;

    if account.data_len() == 0 {
        return Err(ProgramError::AccountDataTooSmall);
    }

    Ok(())
}
