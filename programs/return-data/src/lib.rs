#![no_std]

use abiv2::{
    account::Account, context::InstructionContext, cpi::ReturnData, entrypoint,
    syscall::sol_log_pubkey, ProgramResult,
};
use solana_program_log::log;

entrypoint!(process_instruction);

pub fn process_instruction(
    _context: &InstructionContext,
    _accounts: &mut [Account],
    instruction_data: &[u8],
) -> ProgramResult {
    log!("----");
    let return_data = ReturnData::borrow()?;
    log!("program:");
    unsafe { sol_log_pubkey(return_data.program().as_array().as_ptr()) };
    log!("data len: {}", return_data.as_slice().len());
    log!("----");
    // Make sure the reference is dropped before the mutable borrow.
    drop(return_data);

    let mut return_data = ReturnData::borrow_mut(instruction_data.len())?;
    let data = return_data.as_mut_slice();
    data.copy_from_slice(instruction_data);

    Ok(())
}
