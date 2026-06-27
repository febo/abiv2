//! Types representing account information.
//!
//! Programs have access to two account types:
//!
//! - [`Account`]: these are accounts passed to an instruction. Each account
//!   has a corresponding [`TransactionAccount`].
//!
//! - [`TransactionAccount`]: these are accounts present on a transaction.
//!   Programs have access to all transaction accounts, but can only manipulate
//!   accounts passed to the current instruction (e.g., modify their data or use
//!   them in syscalls).

use {
    crate::{
        context::TRANSACTION_ACCOUNTS_ADDRESS,
        syscall::{assign_owner, set_buffer_length, sol_transfer_lamports},
        MemoryMapping, Ref, RefMut, Volatile, HEAP_ADDRESS, MAX_IMMUTABLE_BORROWS,
        MUTABLY_BORROWED, NOT_BORROWED,
    },
    core::{
        marker::PhantomData,
        ptr::{read_unaligned, read_volatile, NonNull},
    },
    solana_address::Address,
    solana_program_error::{ProgramError, ProgramResult},
};

/// Mask for the signer flag.
const SIGNER_MASK: u32 = 1u32 << 16;

/// Mask for the writable flag.
const WRITABLE_MASK: u32 = 1u32 << 24;

/// Mask for both writable and signer flags.
const WRITABLE_SIGNER_MASK: u32 = WRITABLE_MASK | SIGNER_MASK;

/// Instruction-facing account information.
///
/// `Account` stores the account's transaction index and the access flags
/// supplied to the current instruction. The account metadata and data live in
/// runtime-managed memory, while borrow state is tracked separately so
/// duplicate accounts cannot create conflicting references to the same
/// account data.
///
/// # Invariants
///
/// - The [`TRANSACTION_ACCOUNTS_ADDRESS`] memory region must be available at
///   runtime.
/// - The [`HEAP_ADDRESS`] memory region must be available at runtime, and its
///   first [`crate::BORROW_FLAGS_SIZE`] bytes must be reserved for borrow flags.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Account(u32);

// ABI layout expected by the runtime for `Account`.
const _: () = {
    assert!(align_of::<Account>() == 4);
    assert!(size_of::<Account>() == 4);
};

impl Account {
    /// Create an account from its transaction index and instruction flags.
    #[inline(always)]
    pub const fn new(transaction_index: u16, is_signer: bool, is_writable: bool) -> Self {
        Self(
            transaction_index as u32
                | (SIGNER_MASK * is_signer as u32)
                | (WRITABLE_MASK * is_writable as u32),
        )
    }

    /// Create a read-only, non-signer account for the transaction index.
    #[inline(always)]
    pub const fn readonly(transaction_index: u16) -> Self {
        // No flags are set for a read-only, non-signer account.
        Self(transaction_index as u32)
    }

    /// Create a read-only signer account for the transaction index.
    #[inline(always)]
    pub const fn readonly_signer(transaction_index: u16) -> Self {
        // Set the signer bit while leaving the writable bit clear.
        Self(transaction_index as u32 | SIGNER_MASK)
    }

    /// Create a writable, non-signer account for the transaction index.
    #[inline(always)]
    pub const fn writable(transaction_index: u16) -> Self {
        // Set the writable bit while leaving the signer bit clear.
        Self(transaction_index as u32 | WRITABLE_MASK)
    }

    /// Create a writable signer account for the transaction index.
    #[inline(always)]
    pub const fn writable_signer(transaction_index: u16) -> Self {
        // Set both access flags using the precomputed combined mask.
        Self(transaction_index as u32 | WRITABLE_SIGNER_MASK)
    }

    /// Return the index of the account in the transaction.
    #[inline(always)]
    pub const fn transaction_index(&self) -> u16 {
        self.0 as u16
    }

    /// Return `true` if the account signed the instruction.
    #[inline(always)]
    pub const fn is_signer(&self) -> bool {
        (self.0 & SIGNER_MASK) != 0
    }

    /// Return `true` if the account can be modified by the instruction.
    #[inline(always)]
    pub const fn is_writable(&self) -> bool {
        (self.0 & WRITABLE_MASK) != 0
    }

    /// Return a reference to the corresponding transaction account metadata.
    #[inline(always)]
    pub fn transaction_account(&self) -> &TransactionAccount {
        // SAFETY: The runtime maps transaction account metadata at
        // [`TRANSACTION_ACCOUNTS_ADDRESS`].
        unsafe {
            &*((TRANSACTION_ACCOUNTS_ADDRESS
                + (self.transaction_index() as usize * size_of::<TransactionAccount>()))
                as *const TransactionAccount)
        }
    }

    /// Return a pointer to this account's borrow-state byte.
    ///
    /// # Safety
    ///
    /// The returned pointer may only be dereferenced when the runtime has
    /// reserved account borrow flags.
    #[inline(always)]
    unsafe fn borrow_state(&self) -> *mut u8 {
        (HEAP_ADDRESS + self.transaction_index() as usize) as *mut u8
    }

    /// Return the address of the account.
    #[inline(always)]
    pub fn address(&self) -> &Address {
        &self.transaction_account().address
    }

    /// Changes the owner of the account.
    pub fn assign(&self, program: &Address) {
        assign_owner(self.transaction_index() as u64, program);
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
        self.transaction_account().data.as_slice()
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
        self.transaction_account().data.as_mut_slice()
    }

    /// Checks whether an immutable reference can be created for the account
    /// data, failing if the account is already mutably borrowed or there
    /// are not enough immutable borrows available.
    #[inline(always)]
    fn check_borrow(&self) -> Result<(), ProgramError> {
        // There must be at least one immutable borrow available.
        //
        // SAFETY: The `borrow_state` pointer is valid under `Account`'s
        // runtime memory invariants.
        if unsafe { *self.borrow_state() } >= MAX_IMMUTABLE_BORROWS {
            return Err(ProgramError::AccountBorrowFailed);
        }

        Ok(())
    }

    /// Checks whether a mutable reference can be created for the account data,
    /// failing if the account is already borrowed in any form.
    #[inline(always)]
    fn check_borrow_mut(&self) -> Result<(), ProgramError> {
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
        self.transaction_account().data.len.get() as usize
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

    /// Return the account lamport balance.
    #[inline(always)]
    pub fn lamports(&self) -> u64 {
        self.transaction_account().lamports.get()
    }

    /// Return the address of the program that owns the account.
    #[inline(always)]
    pub fn owner(&self) -> Address {
        self.transaction_account().owner.get()
    }

    /// Return `true` if the account is owned by `program`.
    #[inline(always)]
    pub fn owned_by(&self, program: &Address) -> bool {
        self.transaction_account().owned_by(program)
    }

    /// Transfer lamports to another account.
    #[inline(always)]
    pub fn transfer_lamports(&self, destination: &Account, lamports: u64) {
        sol_transfer_lamports(
            destination.transaction_index() as u64,
            self.transaction_index() as u64,
            lamports,
        );
    }

    /// Resize (either truncating or zero-extending) the account's data.
    #[inline(always)]
    pub fn resize(&mut self, new_len: usize) -> ProgramResult {
        if self.is_borrowed() {
            return Err(ProgramError::AccountBorrowFailed);
        }

        // SAFETY: The account data is not borrowed.
        unsafe { self.resize_unchecked(new_len) };

        Ok(())
    }

    /// Resize (either truncating or zero-extending) the account's data.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the account data is not borrowed
    /// in any form.
    #[inline(always)]
    pub unsafe fn resize_unchecked(&mut self, new_len: usize) {
        set_buffer_length(self.transaction_account().data.ptr as u64, new_len as u64);
    }

    /// Tries to get an immutable reference to the account data, failing if the
    /// account is already mutably borrowed.
    pub fn try_borrow(&self) -> Result<Ref<'_, [u8]>, ProgramError> {
        // Check whether the account data can be borrowed.
        self.check_borrow()?;

        let borrow_state = unsafe { self.borrow_state() };
        // Use one immutable borrow for the account data by incrementing the
        // data borrow counter; we are guaranteed that there is at least one
        // immutable borrow available.
        //
        // SAFETY: The `borrow_state` is a mutable pointer to the borrow state
        // of the account, which is guaranteed to be valid.
        unsafe { *borrow_state += 1 };

        // Return the reference to data.
        Ok(Ref {
            value: NonNull::from(self.transaction_account().data.as_slice()),
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
            // SAFETY: The runtime maps account data as mutable for accounts that can be
            // manipulated by a program.
            value: NonNull::from(unsafe { self.transaction_account().data.as_mut_slice() }),
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

impl TransactionAccount {
    /// Return the account lamport balance.
    #[inline(always)]
    pub fn lamports(&self) -> u64 {
        self.lamports.get()
    }

    /// Return the address of the program that owns the account.
    #[inline(always)]
    pub fn owner(&self) -> Address {
        self.owner.get()
    }

    /// Return `true` if the account is owned by `program`.
    #[inline(always)]
    pub fn owned_by(&self, program: &Address) -> bool {
        let owner_ptr = &raw const self.owner as *const u64;
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
}

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
    fn test_readonly_account_flags() {
        let view = Account::readonly(42);

        assert_eq!(view.transaction_index(), 42);
        assert!(!view.is_signer());
        assert!(!view.is_writable());
    }

    #[test]
    fn test_readonly_signer_account_flags() {
        let view = Account::readonly_signer(42);

        assert_eq!(view.transaction_index(), 42);
        assert!(view.is_signer());
        assert!(!view.is_writable());
    }

    #[test]
    fn test_writable_account_flags() {
        let view = Account::writable(420);

        assert_eq!(view.transaction_index(), 420);
        assert!(!view.is_signer());
        assert!(view.is_writable());
    }

    #[test]
    fn test_writable_signer_account_flags() {
        let view = Account::writable_signer(420);

        assert_eq!(view.transaction_index(), 420);
        assert!(view.is_signer());
        assert!(view.is_writable());
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

        let view = Account::writable_signer(0);

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
