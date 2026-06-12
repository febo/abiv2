use {
    crate::{
        MemoryMapping, Volatile,
        account::{Account, TransactionAccount},
        instruction::Instruction,
    },
    core::slice::from_raw_parts,
    solana_address::Address,
};

/// Address of the runtime-managed instruction region.
///
/// The region contains a contiguous array of [`Instruction`] values,
/// indexed by instruction order in the transaction. Programs have read-only
/// access to this region.
pub(crate) const INSTRUCTION_ADDRESS: usize = 0x600000000usize;

/// Address of the runtime-managed transaction accounts memory region.
///
/// The region contains a contiguous array of [`TransactionAccount`] values.
/// Accounts are indexed by their position in the transaction. Programs have
/// read-only access to this region.
pub(crate) const TRANSACTION_ACCOUNTS_ADDRESS: usize = 0x500000000usize;

/// Address of the runtime-managed transaction metadata memory region.
///
/// The region contains a [`Context`] value. Programs have read-only access
/// to this region.
pub(crate) const TRANSACTION_CONTEXT_ADDRESS: usize = 0x400000000usize;

/// The transaction execution context provided by the runtime.
#[repr(C)]
pub struct Context {
    return_data: ReturnData,

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

// Layout expected by the runtime for `Context`.
const _: () = {
    assert!(align_of::<Context>() == 8);
    assert!(size_of::<Context>() == 88);
};

impl Context {
    /// Returns the transaction execution context.
    ///
    /// # Safety
    ///
    /// This function is safe to call only when the runtime has mapped
    /// transaction context at [`TRANSACTION_CONTEXT_ADDRESS`].
    pub const unsafe fn get() -> &'static Self {
        unsafe { &*(TRANSACTION_CONTEXT_ADDRESS as *const _) }
    }

    /// Returns all accounts in the transaction.
    pub fn accounts(&self) -> &[TransactionAccount] {
        unsafe {
            from_raw_parts(
                TRANSACTION_ACCOUNTS_ADDRESS as _,
                self.account_count as usize,
            )
        }
    }

    /// Returns the number of CPIs in the transaction.
    ///
    /// Includes CPIs that are currently executing or have already completed.
    pub fn cpi_count(&self) -> u16 {
        self.cpi_count_in_trace.get()
    }

    /// Returns the instruction currently being executed.
    pub fn current_instruction(&self) -> &Instruction {
        unsafe {
            &*((INSTRUCTION_ADDRESS
                + (self.current_instruction_index as usize * size_of::<Instruction>()))
                as *const Instruction)
        }
    }

    /// Returns the current number of instructions in the transaction.
    ///
    /// This value is updated by the runtime as CPIs are executed.
    pub fn instruction_count(&self) -> u16 {
        self.instruction_count_in_trace.get()
    }

    /// Returns the current list of instructions.
    ///
    /// This list is updated by the runtime as CPIs are executed.
    pub fn instructions(&self) -> &[Instruction] {
        unsafe { from_raw_parts(INSTRUCTION_ADDRESS as _, self.instruction_count() as usize) }
    }

    /// Returns the transaction fee payer account.
    ///
    /// The fee payer is always the first account in the transaction.
    pub const fn payer(&self) -> &TransactionAccount {
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
    /// Returns the current return-data bytes.
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// Returns the program that last wrote return data.
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
    /// Returns the accounts passed to the CPI being invoked.
    pub fn accounts(&self) -> &[Account] {
        self.accounts.as_slice()
    }

    /// Returns the instruction data passed to the CPI being invoked.
    pub fn instruction_data(&self) -> &[u8] {
        self.instruction_data.as_slice()
    }
}
