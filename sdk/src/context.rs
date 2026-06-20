use {
    crate::{
        account::{Account, TransactionAccount},
        MemoryMapping, Volatile,
    },
    core::slice::from_raw_parts,
    solana_address::Address,
};

/// Address of the runtime-managed instruction region.
///
/// The region contains a contiguous array of [`InstructionContext`] values,
/// indexed by instruction order in the transaction. Programs have read-only
/// access to this region.
pub(crate) const INSTRUCTION_ADDRESS: usize = 0x600000000usize;

/// Address of the runtime-managed transaction accounts memory region.
///
/// The region contains a contiguous array of [`TransactionAccount`] values.
/// Accounts are indexed by their position in the transaction. Programs have
/// read-only access to this region.
pub(crate) const TRANSACTION_ACCOUNTS_ADDRESS: usize = 0x500000000usize;

/// Address of the runtime-managed transaction context memory region.
///
/// The region contains a [`TransactionContext`] value with metadata of the
/// executing transaction. Programs have read-only access to this region.
pub(crate) const TRANSACTION_CONTEXT_ADDRESS: usize = 0x400000000usize;

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
    parent_instruction: u16,

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
        unsafe { from_raw_parts(self.accounts_ptr, self.accounts_len as usize) }
    }

    /// Return the instruction data bytes.
    #[inline(always)]
    pub fn data(&self) -> &[u8] {
        unsafe { from_raw_parts(self.data_ptr, self.data_len as usize) }
    }

    /// Return `true` if the instruction has a parent instruction.
    ///
    /// Top-level instructions do not have a parent instruction.
    #[inline(always)]
    pub const fn has_parent(&self) -> bool {
        self.parent_instruction != u16::MAX
    }

    /// Return the index of the parent instruction if there is one.
    #[inline(always)]
    pub const fn parent(&self) -> Option<u16> {
        if self.has_parent() {
            Some(self.parent_instruction)
        } else {
            None
        }
    }
}

/// The transaction execution context provided by the runtime.
#[repr(C)]
pub struct TransactionContext {
    /// Memory space for the return data.
    return_data: ReturnData,

    /// Memory space for CPI invoke parameters.
    cpi_invoke_params: CpiInvokeParams,

    /// Index of the currently executing instruction.
    pub current_instruction_index: u16,

    /// Total number of instructions in the transaction, including CPIs
    /// and top-level instructions.
    ///
    /// This value is updated by the runtime as CPIs are invoked.
    instruction_count_in_trace: Volatile<u16>,

    /// Current number of CPIs in the transaction.
    ///
    /// Includes CPIs that are currently executing or have already completed.
    cpi_count_in_trace: Volatile<u16>,

    /// Total number of accounts in the transaction.
    pub account_count: u16,
}

// Layout expected by the runtime for `TransactionContext`.
const _: () = {
    assert!(align_of::<TransactionContext>() == 8);
    assert!(size_of::<TransactionContext>() == 88);
};

impl TransactionContext {
    /// Return the transaction execution context.
    ///
    /// # Safety
    ///
    /// This function is safe to call only when the runtime has mapped
    /// transaction context at [`TRANSACTION_CONTEXT_ADDRESS`].
    pub const unsafe fn get() -> &'static Self {
        unsafe { &*(TRANSACTION_CONTEXT_ADDRESS as *const _) }
    }

    /// Return all accounts in the transaction.
    pub fn accounts(&self) -> &[TransactionAccount] {
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
        self.cpi_count_in_trace.get()
    }

    /// Return the instruction currently being executed.
    pub fn current_instruction(&self) -> &InstructionContext {
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
        self.instruction_count_in_trace.get()
    }

    /// Return the current list of instructions.
    ///
    /// This list is updated by the runtime as CPIs are executed.
    pub fn instructions(&self) -> &[InstructionContext] {
        unsafe { from_raw_parts(INSTRUCTION_ADDRESS as _, self.instruction_count() as usize) }
    }

    /// Return the transaction fee payer account.
    ///
    /// The fee payer is always the first account in the transaction.
    pub const fn payer() -> &'static TransactionAccount {
        unsafe { &*(TRANSACTION_ACCOUNTS_ADDRESS as *const _) }
    }
}

/// Runtime return-data state.
#[repr(C)]
pub struct ReturnData {
    /// Address of the program that last wrote data to the scratchpad.
    program: Volatile<Address>,

    /// Return-data bytes.
    data: MemoryMapping<u8>,
}

// Layout expected by the runtime for `ReturnData`.
const _: () = {
    assert!(align_of::<ReturnData>() == 8);
    assert!(size_of::<ReturnData>() == 48);
};

impl ReturnData {
    /// Return the current return-data bytes.
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// Return the program that last wrote return data.
    #[inline(always)]
    pub fn program(&self) -> Address {
        self.program.get()
    }
}

/// Runtime scratch area for CPI invocation parameters.
#[repr(C)]
pub struct CpiInvokeParams {
    /// Accounts passed to the CPI being invoked.
    accounts: MemoryMapping<Account>,

    /// Instruction data passed to the CPI being invoked.
    instruction_data: MemoryMapping<u8>,
}

// Layout expected by the runtime for `CpiInvokeParams`.
const _: () = {
    assert!(align_of::<CpiInvokeParams>() == 8);
    assert!(size_of::<CpiInvokeParams>() == 32);
};

impl CpiInvokeParams {
    /// Return the accounts passed to the CPI being invoked.
    pub fn accounts(&self) -> &[Account] {
        self.accounts.as_slice()
    }

    /// Return the instruction data passed to the CPI being invoked.
    pub fn instruction_data(&self) -> &[u8] {
        self.instruction_data.as_slice()
    }
}
