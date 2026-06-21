#![no_std]

use abiv2::{
    account::Account, context::InstructionContext, entrypoint, error::ProgramError, Address,
    ProgramResult,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _context: &InstructionContext,
    accounts: &[Account],
    instruction_data: &[u8],
) -> ProgramResult {
    let address =
        Address::try_from(instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;

    let [account, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    account.assign(&address);

    Ok(())
}
