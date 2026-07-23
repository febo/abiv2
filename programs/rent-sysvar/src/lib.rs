//! An ABIv2 program that demonstrates how to access rent sysvar information.

#![no_std]

use abiv2::{
    account::Account, context::InstructionContext, entrypoint, error::ProgramError,
    sysvar::rent::Rent, ProgramResult,
};

entrypoint!(process_instruction);

/// Rent required for an empty account.
const EMPTY_ACCOUNT_RENT: u64 = 890880;

pub fn process_instruction(
    _context: &InstructionContext,
    accounts: &[Account],
    _instruction_data: &[u8],
) -> ProgramResult {
    let &[rent_account, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let rent = Rent::get()?;
    let rent_data = rent_account.try_borrow()?;

    assert!(rent_data.len() >= 8);
    assert!(rent_data[..8] == rent.lamports_per_byte.to_le_bytes());
    assert!(rent.try_minimum_balance(0)? == EMPTY_ACCOUNT_RENT);

    Ok(())
}
