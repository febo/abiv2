//! Runtime-mapped transaction and instruction context.
//!
//! The runtime exposes transaction and instruction metadata via
//! [`TransactionContext`] and [`InstructionContext`].

use {
    crate::{
        account::{Account, TransactionAccount},
        Volatile,
    },
    core::slice::from_raw_parts,
};

/// Address of the runtime-managed instruction region.
///
/// The region contains a contiguous array of [`InstructionContext`] values,
/// indexed by instruction order in the transaction. Programs have read-only
/// access to this region.
pub(crate) const INSTRUCTION_ADDRESS: usize = 0x600000000usize;

/// Address of the runtime-managed return data scratch pad.
///
/// Programs have read-only access to this region, unless the `set_buffer_length`
/// syscall is used to specify its length and gain mutable access.
pub const RETURN_DATA_ADDRESS: usize = TRANSACTION_METADATA_ADDRESS;

/// Address of the runtime-managed transaction accounts memory region.
///
/// The region contains a contiguous array of [`TransactionAccount`] values.
/// Accounts are indexed by their position in the transaction. Programs have
/// read-only access to this region.
pub(crate) const TRANSACTION_ACCOUNTS_ADDRESS: usize = 0x500000000usize;

/// Address of the runtime-managed transaction metadata memory region.
pub(crate) const TRANSACTION_METADATA_ADDRESS: usize = 0x400000000usize;

/// Address of the transaction context data.
///
/// The address corresponds to a [`TransactionContext`] value for the executing
/// transaction. Programs have read-only access to this region.
pub(crate) const TRANSACTION_CONTEXT_ADDRESS: usize = TRANSACTION_METADATA_ADDRESS + 0x50usize;

/// The instruction execution context provided by the runtime.
#[repr(C)]
pub struct InstructionContext {
    /// Reserved for future use.
    _reserved: u16,

    /// Index of the program account being executed.
    pub program_account_index: u16,

    /// Cross-program invocation nesting level for this instruction.
    pub nesting_level: u16,

    /// Index of the parent instruction in the transaction.
    ///
    /// This value is set to `u16::MAX` for top-level instructions.
    parent_instruction_index: u16,

    /// Pointer to the accounts passed to the instruction.
    accounts_ptr: *const Account,

    /// Number of accounts available at `accounts_ptr`.
    accounts_len: u64,

    /// Pointer to the instruction data bytes.
    data_ptr: *const u8,

    /// Length of the instruction data in bytes.
    data_len: u64,
}

// Layout expected by the runtime for `InstructionContext`.
const _: () = {
    assert!(align_of::<InstructionContext>() == 8);
    assert!(size_of::<InstructionContext>() == 40);
};

impl InstructionContext {
    /// Return the accounts passed to this instruction.
    #[inline(always)]
    pub fn accounts(&self) -> &[Account] {
        // SAFETY: The runtime provides `accounts_ptr` and `accounts_len` as a
        // valid account slice for this instruction.
        unsafe { from_raw_parts(self.accounts_ptr, self.accounts_len as usize) }
    }

    /// Return the instruction data bytes.
    #[inline(always)]
    pub fn data(&self) -> &[u8] {
        // SAFETY: The runtime provides `data_ptr` and `data_len` as a valid
        // byte slice for this instruction.
        unsafe { from_raw_parts(self.data_ptr, self.data_len as usize) }
    }

    /// Return `true` if the instruction has a parent instruction.
    #[inline(always)]
    pub const fn has_parent(&self) -> bool {
        self.parent_instruction_index != u16::MAX
    }

    /// Return the parent instruction if there is one.
    #[inline(always)]
    pub const fn parent(&self) -> Option<&'static InstructionContext> {
        if self.has_parent() {
            // SAFETY: The runtime maps instruction metadata at
            // [`INSTRUCTION_ADDRESS`].
            Some(unsafe {
                &*((INSTRUCTION_ADDRESS
                    + (self.parent_instruction_index as usize * size_of::<InstructionContext>()))
                    as *const _)
            })
        } else {
            None
        }
    }

    /// Return the transaction account for the executing program.
    #[inline(always)]
    pub const fn program(&self) -> &'static TransactionAccount {
        // SAFETY: The runtime maps transaction account metadata at
        // [`TRANSACTION_ACCOUNTS_ADDRESS`].
        unsafe {
            &*((TRANSACTION_ACCOUNTS_ADDRESS
                + (self.program_account_index as usize * size_of::<TransactionAccount>()))
                as *const _)
        }
    }
}

/// The transaction execution context provided by the runtime.
#[repr(C)]
pub struct TransactionContext {
    /// Index of the currently executing instruction.
    pub current_instruction_index: u16,

    /// Total number of instructions in the transaction, including CPIs
    /// and top-level instructions.
    ///
    /// This value is updated by the runtime as CPIs are invoked.
    instruction_count: Volatile<u16>,

    /// Current number of CPIs in the transaction.
    ///
    /// Includes CPIs that are currently executing or have already completed.
    cpi_count: Volatile<u16>,

    /// Total number of accounts in the transaction.
    pub account_count: u16,
}

// Layout expected by the runtime for `TransactionContext`.
const _: () = {
    assert!(align_of::<TransactionContext>() == 2);
    assert!(size_of::<TransactionContext>() == 8);
};

impl TransactionContext {
    /// Return the transaction execution context.
    pub const fn get() -> &'static Self {
        // SAFETY: The runtime maps a `TransactionContext` at this fixed
        // transaction context address.
        unsafe { &*(TRANSACTION_CONTEXT_ADDRESS as *const _) }
    }

    /// Return all accounts in the transaction.
    pub fn accounts(&self) -> &[TransactionAccount] {
        // SAFETY: The runtime maps transaction account metadata at
        // [`TRANSACTION_ACCOUNTS_ADDRESS`].
        unsafe {
            from_raw_parts(
                TRANSACTION_ACCOUNTS_ADDRESS as _,
                self.account_count as usize,
            )
        }
    }

    /// Return the number of CPIs in the transaction.
    ///
    /// Includes CPIs that are currently executing or have already completed.
    pub fn cpi_count(&self) -> u16 {
        self.cpi_count.get()
    }

    /// Return the instruction currently being executed.
    pub fn current_instruction(&self) -> &InstructionContext {
        // SAFETY: The runtime maps instruction metadata at
        // [`INSTRUCTION_ADDRESS`].
        unsafe {
            &*((INSTRUCTION_ADDRESS
                + (self.current_instruction_index as usize * size_of::<InstructionContext>()))
                as *const InstructionContext)
        }
    }

    /// Return the current number of instructions in the transaction.
    ///
    /// This value is updated by the runtime as CPIs are executed.
    pub fn instruction_count(&self) -> u16 {
        self.instruction_count.get()
    }

    /// Return the current list of instructions.
    ///
    /// This list is updated by the runtime as CPIs are executed.
    pub fn instructions(&self) -> &[InstructionContext] {
        // SAFETY: The runtime maps instruction metadata at
        // [`INSTRUCTION_ADDRESS`].
        unsafe { from_raw_parts(INSTRUCTION_ADDRESS as _, self.instruction_count() as usize) }
    }

    /// Return the transaction fee payer account.
    ///
    /// The fee payer is always the first account in the transaction.
    pub const fn payer() -> &'static TransactionAccount {
        // SAFETY: The runtime maps transaction account metadata at
        // [`TRANSACTION_ACCOUNTS_ADDRESS`].
        unsafe { &*(TRANSACTION_ACCOUNTS_ADDRESS as *const _) }
    }
}
