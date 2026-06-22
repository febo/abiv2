use abiv2::{
    account::Account, context::InstructionContext, default_panic_handler, deny_allocator,
    error::ProgramError, program_entrypoint, ProgramResult,
};

program_entrypoint!(process_instruction);
default_panic_handler!();
deny_allocator!();

pub fn process_instruction(
    _context: &InstructionContext,
    accounts: &mut [Account],
    _instruction_data: &[u8],
) -> ProgramResult {
    let [account, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // This should panic at runtime.
    let addresses = vec![account.address()];
    core::hint::black_box(addresses);

    Ok(())
}
