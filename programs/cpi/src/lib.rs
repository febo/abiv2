//! An ABIv2 program that demonstrates how to perform cross-program invocations (CPI)
//! to other programs.
//!
//! The program supports three instructions:
//!   1. `CreateAccount`: Creates a new account by invoking the system program.
//!   2. `Trace`: Logs instruction and transaction information by invoking a trace program.
//!   3. `WriteReturn`: Writes data as return data by invoking a return data program.

#![no_std]

use core::ptr::copy_nonoverlapping;

use abiv2::{
    account::Account,
    address::ADDRESS_BYTES,
    context::InstructionContext,
    cpi::{Parameters, ReturnData, Signer},
    entrypoint,
    error::ProgramError,
    syscall::sol_invoke_signed,
    Address, ProgramResult,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    context: &InstructionContext,
    accounts: &[Account],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.first() {
        Some(&0) => {
            let &[from, to, system_program, ..] = accounts else {
                return Err(ProgramError::NotEnoughAccountKeys);
            };

            CreateAccount {
                from,
                to,
                system_program,
                lamports: 1_000_000_000,
                space: 100,
                owner: &context.program_account().address,
            }
            .invoke()
        }
        Some(&1) => {
            let &[trace_program, ..] = accounts else {
                return Err(ProgramError::NotEnoughAccountKeys);
            };

            Trace { trace_program }.invoke()
        }
        Some(&2) => {
            let &[return_data_program, ..] = accounts else {
                return Err(ProgramError::NotEnoughAccountKeys);
            };

            WriteReturn {
                return_data_program: return_data_program,
                data: context.program_account().address.as_array(),
            }
            .invoke()?;

            let return_data = ReturnData::borrow()?;

            if &return_data.program() != return_data_program.address()
                || return_data.as_slice() != context.program_account().address.as_array()
            {
                return Err(ProgramError::InvalidArgument);
            }

            Ok(())
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

/// Create a new account.
///
/// Accounts expected by this instruction:
///
///   0. `[writable, signer]` Funding account.
///   1. `[writable, signer]` New account.
///
/// Data expected by this instruction:
///
///   - `u64` Number of lamports to transfer to the new account.
///   - `u64` Number of bytes of memory to allocate.
///   - `Address` Address of the program that will own the new account.
pub struct CreateAccount<'address> {
    /// Callee program account.
    pub system_program: Account,

    /// Funding account.
    pub from: Account,

    /// New account.
    pub to: Account,

    /// Number of lamports to transfer to the new account.
    pub lamports: u64,

    /// Number of bytes of memory to allocate.
    pub space: u64,

    /// Address of program that will own the new account.
    pub owner: &'address Address,
}

impl CreateAccount<'_> {
    pub const DISCRIMINATOR: u32 = 0;

    #[inline(always)]
    pub fn invoke(&self) -> ProgramResult {
        self.invoke_signed(&[])
    }

    #[inline(always)]
    pub fn invoke_signed(&self, signers: &[Signer]) -> ProgramResult {
        if self.from.is_borrowed() | self.to.is_borrowed() {
            return Err(ProgramError::AccountBorrowFailed);
        }

        let mut parameters = Parameters::for_invocation(2, 52)?;

        // Accounts.
        let accounts = parameters.accounts_mut();
        accounts[0] = Account::writable_signer(self.from.transaction_index());
        accounts[1] = Account::writable_signer(self.to.transaction_index());

        // Instruction data:
        // - [ 0..4 ]: instruction discriminator
        // - [ 4..12]: lamports
        // - [12..20]: account space
        // - [20..52]: owner address
        let instruction_data = parameters.instruction_data_mut();
        // SAFETY: All writes are within bounds of the allocated data.
        unsafe {
            let dst = instruction_data.as_mut_ptr();

            copy_nonoverlapping(
                Self::DISCRIMINATOR.to_le_bytes().as_ptr(),
                dst,
                size_of::<u32>(),
            );

            copy_nonoverlapping(
                self.lamports.to_le_bytes().as_ptr(),
                dst.add(4),
                size_of::<u64>(),
            );

            copy_nonoverlapping(
                self.space.to_le_bytes().as_ptr(),
                dst.add(12),
                size_of::<u64>(),
            );

            copy_nonoverlapping(self.owner.as_ref().as_ptr(), dst.add(20), ADDRESS_BYTES);
        }

        sol_invoke_signed(
            self.system_program.as_ref().transaction_index() as u64,
            signers.as_ptr() as u64,
            signers.len() as u64,
        );

        Ok(())
    }
}

/// Logs instruction and transacrion information.
pub struct Trace {
    /// Callee program account.
    pub trace_program: Account,
}

impl Trace {
    #[inline(always)]
    pub fn invoke(&self) -> ProgramResult {
        sol_invoke_signed(self.trace_program.as_ref().transaction_index() as u64, 0, 0);

        Ok(())
    }
}

/// Write the data as a return data.
pub struct WriteReturn<'data> {
    /// Callee program account.
    pub return_data_program: Account,

    /// The return data to write.
    pub data: &'data [u8],
}

impl WriteReturn<'_> {
    #[inline(always)]
    pub fn invoke(&self) -> ProgramResult {
        let mut parameters = Parameters::for_invocation(0, self.data.len())?;

        let instruction_data = parameters.instruction_data_mut();
        instruction_data.copy_from_slice(self.data);

        sol_invoke_signed(
            self.return_data_program.as_ref().transaction_index() as u64,
            0,
            0,
        );

        Ok(())
    }
}
