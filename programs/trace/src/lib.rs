#![no_std]

use abiv2::{
    account::Account,
    context::{InstructionContext, TransactionContext},
    entrypoint, ProgramResult,
};
use solana_program_log::log;

entrypoint!(process_instruction);

pub fn process_instruction(
    context: &InstructionContext,
    _accounts: &mut [Account],
    _instruction_data: &[u8],
) -> ProgramResult {
    // Instruction context.

    log!("---- Instruction");
    log!("Program account index: {}", context.program_account_index);
    log!("Nesting level: {}", context.nesting_level);
    log!("Is top-level: {}", !context.has_parent());

    // Transaction context.

    let transaction = TransactionContext::get();

    log!("---- Transaction");
    log!(
        "Current instruction index: {}",
        transaction.current_instruction_index
    );
    log!("Instruction count: {}", transaction.instruction_count());
    log!("CPI count: {}", transaction.cpi_count());
    log!("Accounts count: {}", transaction.account_count);
    log!("----");

    Ok(())
}
