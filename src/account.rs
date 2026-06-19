use {
    crate::{
        HEAP_ADDRESS, MUTABLY_BORROWED, MemoryMapping, NOT_BORROWED, Ref, RefMut, Volatile,
        context::TRANSACTION_ACCOUNTS_ADDRESS,
    },
    core::{
        marker::PhantomData,
        ptr::{NonNull, read_unaligned, read_volatile},
    },
    solana_address::Address,
    solana_program_error::ProgramError,
};

/// Instruction-facing view of a transaction account.
///
/// `Account` stores the account's transaction index plus the access flags
/// supplied to the current instruction. The account metadata and data live in
/// runtime-managed memory, while borrow state is tracked separately so
/// duplicate account views cannot create conflicting references to the same
/// account data.
///
/// # Invariants
///
/// - The [`TRANSACTION_ACCOUNTS_ADDRESS`] memory region must be available at
///   runtime.
/// - The [`HEAP_ADDRESS`] memory region must be available at runtime, and its
///   first `4096` bytes must be reserved for account borrow flags.
pub struct Account {
    /// The index of the account in the transaction's account list.
    pub transaction_index: u16,

    /// Signer flag for this instruction.
    ///
    /// Nonzero when the account is a signer in the instruction.
    signer: u8,

    /// Writable flag for this instruction.
    ///
    /// Nonzero when the account can be modified by the instruction.
    writable: u8,
}

// ABI layout expected by the runtime for `Account`.
const _: () = {
    assert!(align_of::<Account>() == 2);
    assert!(size_of::<Account>() == 4);
};

impl Account {
    /// Return a pointer to the corresponding transaction account metadata.
    ///
    /// # Safety
    ///
    /// The returned pointer may only be dereferenced when the runtime has
    /// mapped transaction account metadata at [`TRANSACTION_ACCOUNTS_ADDRESS`].
    #[inline(always)]
    unsafe fn transaction_account(&self) -> *const TransactionAccount {
        (TRANSACTION_ACCOUNTS_ADDRESS
            + (self.transaction_index as usize * size_of::<TransactionAccount>()))
            as *const TransactionAccount
    }

    /// Return a pointer to this account's borrow-state byte.
    ///
    /// # Safety
    ///
    /// The returned pointer may only be dereferenced when the runtime has
    /// reserved account borrow flags.
    #[inline(always)]
    unsafe fn borrow_state(&self) -> *mut u8 {
        (HEAP_ADDRESS + self.transaction_index as usize) as *mut u8
    }

    /// Return the address of the account.
    #[inline(always)]
    pub fn address(&self) -> &Address {
        // SAFETY: The `transaction_account` pointer is valid under `Account`'s
        // runtime memory invariants.
        unsafe { &(*self.transaction_account()).address }
    }

    /// Return an immutable reference to the data in the account.
    ///
    /// # Safety
    ///
    /// This method does not update the borrow flag. The caller must ensure that
    /// no mutable borrow exists for the same account data for the duration of
    /// the returned slice. Useful when an instruction has verified
    /// non-duplicate accounts.
    #[inline(always)]
    pub unsafe fn borrow_unchecked(&self) -> &[u8] {
        // SAFETY: The `transaction_account` pointer is valid under
        // `Account`'s runtime memory invariants.
        unsafe { (*self.transaction_account()).data.as_slice() }
    }

    /// Return a mutable reference to the data in the account.
    ///
    /// # Safety
    ///
    /// This method does not update the borrow flag. The caller must ensure that
    /// no other borrow exists for the same account data for the duration of the
    /// returned slice. Useful when an instruction has verified non-duplicate
    /// accounts.
    #[allow(clippy::mut_from_ref)]
    #[inline(always)]
    pub unsafe fn borrow_unchecked_mut(&mut self) -> &mut [u8] {
        // SAFETY: The `transaction_account` pointer is valid under
        // `Account`'s runtime memory invariants.
        unsafe { (*self.transaction_account()).data.as_mut_slice() }
    }

    /// Checks whether an immutable reference can be created for the account
    /// data, failing if the account is already mutably borrowed or there
    /// are not enough immutable borrows available.
    #[inline(always)]
    pub fn check_borrow(&self) -> Result<(), ProgramError> {
        // There must be at least one immutable borrow available, so we test
        // against `MUTABLY_BORROWED - 2`, which covers the case when there are
        // no immutable borrows available (`MUTABLY_BORROWED - 1`) and the case
        // when the account is mutably borrowed (`MUTABLY_BORROWED`).
        //
        // SAFETY: The `borrow_state` pointer is valid under `Account`'s
        // runtime memory invariants.
        if unsafe { *self.borrow_state() } > (MUTABLY_BORROWED - 2) {
            return Err(ProgramError::AccountBorrowFailed);
        }

        Ok(())
    }

    /// Checks whether a mutable reference can be created for the account data,
    /// failing if the account is already borrowed in any form.
    #[inline(always)]
    pub fn check_borrow_mut(&self) -> Result<(), ProgramError> {
        // SAFETY: The `borrow_state` pointer is valid under `Account`'s
        // runtime memory invariants.
        if unsafe { *self.borrow_state() } != NOT_BORROWED {
            return Err(ProgramError::AccountBorrowFailed);
        }

        Ok(())
    }

    /// Return the length of the account data in bytes.
    #[inline(always)]
    pub fn data_len(&self) -> usize {
        // SAFETY: The `transaction_account` pointer is valid under
        // `Account`'s runtime memory invariants.
        unsafe { (*self.transaction_account()).data.len.get() as usize }
    }

    /// Return `true` if the account data is borrowed in any form.
    #[inline(always)]
    pub fn is_borrowed(&self) -> bool {
        unsafe { *self.borrow_state() != NOT_BORROWED }
    }

    /// Return `true` if the account data is empty.
    ///
    /// An account is considered empty if the data length is zero.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.data_len() == 0
    }

    /// Return `true` if the account data is mutably borrowed.
    #[inline(always)]
    pub fn is_mutably_borrowed(&self) -> bool {
        unsafe { *self.borrow_state() == MUTABLY_BORROWED }
    }

    /// Return `true` if the account signed the instruction.
    #[inline(always)]
    pub fn is_signer(&self) -> bool {
        self.signer != 0
    }

    /// Return `true` if the account can be modified by the instruction.
    #[inline(always)]
    pub fn is_writable(&self) -> bool {
        self.writable != 0
    }

    /// Return the account lamport balance.
    #[inline(always)]
    pub fn lamports(&self) -> u64 {
        // SAFETY: The `transaction_account` pointer is valid under
        // `Account`'s runtime memory invariants.
        unsafe { (*self.transaction_account()).lamports.get() }
    }

    /// Return the address of the program that owns the account.
    #[inline(always)]
    pub fn owner(&self) -> Address {
        // SAFETY: The `transaction_account` pointer is valid under
        // `Account`'s runtime memory invariants.
        unsafe { (*self.transaction_account()).owner.get() }
    }

    /// Return `true` if the account is owned by `program`.
    #[inline(always)]
    pub fn owned_by(&self, program: &Address) -> bool {
        // SAFETY: The `transaction_account` pointer is valid under
        // `Account`'s runtime memory invariants.
        let owner_ptr = unsafe { &raw const (*self.transaction_account()).owner } as *const u64;
        let program_ptr = program.as_array().as_ptr() as *const u64;

        // SAFETY: Both pointers are valid for 32 bytes. `owner_ptr` is aligned
        // to 8 bytes by the type layout, while `program_ptr` may be unaligned
        // because it points into a byte array.
        unsafe {
            read_volatile(owner_ptr) == read_unaligned(program_ptr)
                && read_volatile(owner_ptr.add(1)) == read_unaligned(program_ptr.add(1))
                && read_volatile(owner_ptr.add(2)) == read_unaligned(program_ptr.add(2))
                && read_volatile(owner_ptr.add(3)) == read_unaligned(program_ptr.add(3))
        }
    }

    /// Tries to get an immutable reference to the account data, failing if the
    /// account is already mutably borrowed.
    pub fn try_borrow(&self) -> Result<Ref<'_, [u8]>, ProgramError> {
        // Check whether the account data can be borrowed.
        self.check_borrow()?;

        let borrow_state = unsafe { self.borrow_state() };
        // Use one immutable borrow for data by incrementing the data borrow
        // counter; we are guaranteed that there is at least one immutable
        // borrow available.
        //
        // SAFETY: The `borrow_state` is a mutable pointer to the borrow state
        // of the account, which is guaranteed to be valid.
        unsafe { *borrow_state += 1 };

        // Return the reference to data.
        Ok(Ref {
            value: unsafe { NonNull::from((*self.transaction_account()).data.as_slice()) },
            state: unsafe { NonNull::new_unchecked(borrow_state) },
            marker: PhantomData,
        })
    }

    /// Tries to get a mutable reference to the account data, failing if the
    /// account is already borrowed in any form.
    pub fn try_borrow_mut(&mut self) -> Result<RefMut<'_, [u8]>, ProgramError> {
        // Check whether the account data can be mutably borrowed.
        self.check_borrow_mut()?;

        let borrow_state = unsafe { self.borrow_state() };
        // Set the borrow state to the mutable-borrow sentinel; we are
        // guaranteed that the account data is not already borrowed in any form.
        //
        // SAFETY: The `borrow_state` is a mutable pointer to the borrow state
        // of the account, which is guaranteed to be valid.
        unsafe { *borrow_state = MUTABLY_BORROWED };

        // Return the mutable reference to data.
        Ok(RefMut {
            value: unsafe { NonNull::from((*self.transaction_account()).data.as_mut_slice()) },
            state: unsafe { NonNull::new_unchecked(borrow_state) },
            marker: PhantomData,
        })
    }
}

/// Allows `Account` to be used as a reference to itself
/// for convenience.
impl AsRef<Account> for Account {
    #[inline(always)]
    fn as_ref(&self) -> &Account {
        self
    }
}

/// Runtime metadata for an account in a transaction.
#[repr(C)]
pub struct TransactionAccount {
    /// The address of the account.
    pub address: Address,

    /// The program that owns the account.
    owner: Volatile<Address>,

    /// The account lamport balance.
    lamports: Volatile<u64>,

    /// Mapped account data payload.
    data: MemoryMapping<u8>,
}

// ABI layout expected by the runtime for `TransactionAccount`.
const _: () = {
    assert!(align_of::<TransactionAccount>() == 8);
    assert!(size_of::<TransactionAccount>() == 88);
};

#[cfg(all(test, unix))]
mod tests {
    use {
        super::*,
        core::ptr::{copy_nonoverlapping, write},
    };

    const ACCOUNT_DATA_ADDRESS: usize = 0x800000000;

    const ACCOUNT_DATA_PAGE_SIZE: usize = 1024;

    const TEST_REGION_LEN: usize = 16 * 1024;

    #[cfg(target_os = "linux")]
    const MAP_ANON_FLAG: libc::c_int = libc::MAP_ANONYMOUS;

    #[cfg(not(target_os = "linux"))]
    const MAP_ANON_FLAG: libc::c_int = libc::MAP_ANON;

    struct MappedRegion {
        ptr: *mut libc::c_void,
        len: usize,
    }

    impl Drop for MappedRegion {
        fn drop(&mut self) {
            let result = unsafe { libc::munmap(self.ptr, self.len) };
            debug_assert_eq!(result, 0);
        }
    }

    unsafe fn map_region(address: usize, len: usize) -> MappedRegion {
        let ptr = unsafe {
            libc::mmap(
                address as *mut libc::c_void,
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | MAP_ANON_FLAG | libc::MAP_FIXED,
                -1,
                0,
            )
        };

        assert_ne!(
            ptr,
            libc::MAP_FAILED,
            "failed to map test region at {address:#x}"
        );
        assert_eq!(ptr as usize, address);

        MappedRegion { ptr, len }
    }

    struct Memory {
        _accounts: MappedRegion,
        _borrow_states: MappedRegion,
        _account_data: MappedRegion,
    }

    struct TestAccount<'data> {
        address: Address,
        owner: Address,
        lamports: u64,
        data: &'data [u8],
    }

    unsafe fn transaction_account_ptr(index: usize) -> *mut TransactionAccount {
        (TRANSACTION_ACCOUNTS_ADDRESS + (index * size_of::<TransactionAccount>()))
            as *mut TransactionAccount
    }

    fn prepare_accounts(accounts: &[TestAccount]) -> Memory {
        let memory = Memory {
            _accounts: unsafe { map_region(TRANSACTION_ACCOUNTS_ADDRESS, TEST_REGION_LEN) },
            _borrow_states: unsafe { map_region(HEAP_ADDRESS, TEST_REGION_LEN) },
            _account_data: unsafe { map_region(ACCOUNT_DATA_ADDRESS, TEST_REGION_LEN) },
        };

        unsafe {
            write(HEAP_ADDRESS as *mut u8, NOT_BORROWED);

            for (index, account) in accounts.iter().enumerate() {
                let account_data_address = ACCOUNT_DATA_ADDRESS + index * ACCOUNT_DATA_PAGE_SIZE;
                // Initialize transaction account metadata.
                write(
                    transaction_account_ptr(index),
                    TransactionAccount {
                        address: account.address,
                        owner: Volatile::new(account.owner),
                        lamports: Volatile::new(account.lamports),
                        data: MemoryMapping {
                            ptr: account_data_address as *const u8,
                            len: Volatile::new(account.data.len() as u64),
                        },
                    },
                );
                copy_nonoverlapping(
                    account.data.as_ptr(),
                    account_data_address as *mut u8,
                    account.data.len(),
                );
            }
        }

        memory
    }

    #[test]
    fn test_borrow_account_data() {
        let address = Address::new_unique();
        let owner = Address::new_unique();
        let data = &[5u8; 168];

        let _memory = prepare_accounts(&[TestAccount {
            address,
            owner,
            lamports: 1_000_000_000,
            data,
        }]);

        let view = Account {
            transaction_index: 0,
            signer: 1,
            writable: 1,
        };

        assert_eq!(view.address(), &address);
        assert_eq!(view.owner(), owner);
        assert!(view.owned_by(&owner));
        assert_eq!(view.lamports(), 1_000_000_000);
        assert_eq!(view.data_len(), data.len());

        assert!(view.is_signer());
        assert!(view.is_writable());
        assert!(!view.is_borrowed());

        let account_data = view.try_borrow().unwrap();
        assert_eq!(account_data.as_ref(), data);
        assert!(view.is_borrowed());
        assert!(view.check_borrow().is_ok());
        assert!(!view.is_mutably_borrowed());
        assert!(view.check_borrow_mut().is_err());

        core::mem::drop(account_data);
        assert!(!view.is_borrowed());
        assert!(view.check_borrow_mut().is_ok());
    }
}
