#![no_std]

use abiv2::{
    account::Account, context::InstructionContext, entrypoint, error::ProgramError, ProgramResult,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _context: &InstructionContext,
    accounts: &mut [Account],
    _instruction_data: &[u8],
) -> ProgramResult {
    let [first, second, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let data1 = first.try_borrow()?;
    // This borrow will fail when the accounts are duplicated.
    let mut data2 = second.try_borrow_mut()?;

    data2.copy_from_slice(&data1);

    Ok(())
}
