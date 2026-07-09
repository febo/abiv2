//! An ABIv2 program that demonstrates how to change the owner of an account.

#![no_std]

use abiv2::{
    account::Account, context::InstructionContext, cpi::ReturnData, entrypoint, ProgramResult,
};
use solana_program_log::log;

entrypoint!(process_instruction);

pub fn process_instruction(
    _context: &InstructionContext,
    _accounts: &[Account],
    instruction_data: &[u8],
) -> ProgramResult {
    // The return data at beginning of the instruction
    // should be empty, with the program address as the
    // current program's address.
    let return_data = ReturnData::borrow()?;
    log!("----");
    log!("program:");
    return_data.program().log();
    log!("data len: {}", return_data.as_slice().len());
    log!("----");

    // Make sure the reference is dropped before the mutable borrow.
    drop(return_data);

    let mut return_data = ReturnData::borrow_mut(instruction_data.len())?;
    let data = return_data.as_mut_slice();
    data.copy_from_slice(instruction_data);

    Ok(())
}
