use {crate::account::Account, core::slice::from_raw_parts};

/// Runtime metadata for an instruction in the transaction.
#[repr(C)]
pub struct Instruction {
    /// Reserved for future use.
    _reserved: u16,

    /// Index of the program account being executed.
    pub program_account_index: u16,

    /// Cross-program invocation nesting level for this instruction.
    pub nesting_level: u16,

    /// Index of the parent instruction in the transaction.
    ///
    /// This value is set to `u16::MAX` for top-level instructions.
    pub parent_instruction: u16,

    /// Pointer to the accounts passed to the instruction.
    accounts_ptr: *const Account,

    /// Number of accounts available at `accounts_ptr`.
    accounts_len: u64,

    /// Pointer to the instruction data bytes.
    data_ptr: *const u8,

    /// Length of the instruction data in bytes.
    data_len: u64,
}

// Layout expected by the runtime for `Instruction`.
const _: () = {
    assert!(align_of::<Instruction>() == 8);
    assert!(size_of::<Instruction>() == 40);
};

impl Instruction {
    /// Returns the accounts passed to this instruction.
    #[inline(always)]
    pub fn accounts(&self) -> &[Account] {
        unsafe { from_raw_parts(self.accounts_ptr, self.accounts_len as usize) }
    }

    /// Returns the instruction data bytes.
    #[inline(always)]
    pub fn data(&self) -> &[u8] {
        unsafe { from_raw_parts(self.data_ptr, self.data_len as usize) }
    }

    /// Returns `true` if this is a top-level instruction.
    #[inline(always)]
    pub const fn is_top_level(&self) -> bool {
        self.parent_instruction == u16::MAX
    }
}
