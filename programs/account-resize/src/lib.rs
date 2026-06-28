#![no_std]

use abiv2::{
    account::Account, context::InstructionContext, entrypoint, error::ProgramError, ProgramResult,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _context: &InstructionContext,
    accounts: &mut [Account],
    instruction_data: &[u8],
) -> ProgramResult {
    let [account, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let new_len = if instruction_data.len() == 8 {
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
