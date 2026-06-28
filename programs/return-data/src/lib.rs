#![no_std]

use abiv2::{
    account::Account, context::InstructionContext, cpi::ReturnData, entrypoint, ProgramResult,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _context: &InstructionContext,
    _accounts: &mut [Account],
    instruction_data: &[u8],
) -> ProgramResult {
    let mut return_data = ReturnData::get_mut(instruction_data.len())?;
    let data = return_data.as_mut_slice();
    data.copy_from_slice(instruction_data);

    Ok(())
}
