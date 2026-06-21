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
    let [from, to, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let amount = if instruction_data.len() == 8 {
        u64::from_le_bytes(unsafe { *(instruction_data.as_ptr() as *const [u8; 8]) })
    } else {
        return Err(ProgramError::InvalidInstructionData);
    };

    from.transfer_lamports(to, amount);

    Ok(())
}
