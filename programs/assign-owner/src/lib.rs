use abiv2::{
    account::Account, context::InstructionContext, error::ProgramError, program_entrypoint,
    Address, ProgramResult,
};

program_entrypoint!(process_instruction);

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
