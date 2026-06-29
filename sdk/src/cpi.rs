//! Types for cross-program invocation.
//!
//! Programs passed parameters to cross-program invocation by writing
//! them at a pre-defined scratchpad memory area. Similarly, programs
//! can read and write return data from a pre-defined scratchpad area
//! reserved for return data.

use {
    crate::{
        account::Account,
        memory::{self, MemoryMapping, CPI_PARAMETERS_ADDRESS, HEAP_ADDRESS, RETURN_DATA_ADDRESS},
        syscall::set_buffer_length,
        Ref, RefMut, Volatile, MAX_IMMUTABLE_BORROWS, MUTABLY_BORROWED, NOT_BORROWED,
    },
    core::{
        marker::PhantomData,
        ops::Deref,
        ptr::{read_unaligned, read_volatile, NonNull},
        slice::from_raw_parts,
    },
    solana_address::Address,
    solana_program_error::ProgramError,
};

/// Index of the borrow flag for the CPI invoke parameters.
const CPI_BORROW_FLAG_INDEX: usize = 4088;

/// Index of the borrow flag for the return data.
const RETURN_DATA_BORROW_FLAG_INDEX: usize = 4089;

/// Runtime scratchpad area for writing CPI invocation parameters.
#[repr(C)]
pub struct Parameters {
    /// Instruction data passed to the CPI being invoked.
    instruction_data: MemoryMapping<u8>,

    /// Accounts passed to the CPI being invoked.
    accounts: MemoryMapping<Account>,
}

// Layout expected by the runtime for `Parameters`.
const _: () = {
    assert!(align_of::<Parameters>() == 8);
    assert!(size_of::<Parameters>() == 32);
};

impl Parameters {
    /// Returns the runtime scratchpad area parameters for a cross-program invocation.
    ///
    /// This resizes the instruction data and accounts scratchpads and
    /// returns a mutable reference to the fixed runtime-managed parameters region.
    pub fn for_invocation(
        accounts_len: usize,
        data_len: usize,
    ) -> Result<RefMut<'static, Self>, ProgramError> {
        unsafe {
            let borrow_state = memory::mut_ptr_at(HEAP_ADDRESS + CPI_BORROW_FLAG_INDEX);

            if *borrow_state != NOT_BORROWED {
                return Err(ProgramError::Immutable);
            }

            *borrow_state = MUTABLY_BORROWED;

            Ok(RefMut {
                value: NonNull::from(Self::for_invocation_unchecked(accounts_len, data_len)),
                state: NonNull::new_unchecked(borrow_state),
                marker: PhantomData,
            })
        }
    }

    /// Returns the runtime scratchpad area parameters for a cross-program invocation.
    ///
    /// This resizes the instruction data and accounts scratchpads and returns a mutable
    /// reference to the fixed runtime-managed parameters region.
    ///
    /// It behaves similarly to [`Self::for_invocation`] but does not track borrows.
    ///
    /// # Safety
    ///
    /// The caller must ensure that no other reference to the parameters region is
    /// active while the returned mutable reference is used. Calling this function
    /// again before a previously returned reference is no longer used violates Rust
    /// aliasing rules regarding mutable references and cause undefined behaviour.
    pub unsafe fn for_invocation_unchecked(
        accounts_len: usize,
        data_len: usize,
    ) -> &'static mut Parameters {
        let parameters = unsafe { memory::mut_ref_at::<Parameters>(CPI_PARAMETERS_ADDRESS) };

        set_buffer_length(
            parameters.accounts.ptr as u64,
            (accounts_len * size_of::<Account>()) as u64,
        );

        set_buffer_length(parameters.instruction_data.ptr as u64, data_len as u64);

        parameters
    }

    /// Return the accounts passed to the CPI being invoked.
    pub fn accounts(&self) -> &[Account] {
        self.accounts.as_slice()
    }

    /// Return a mutable accounts slice for a CPI.
    ///
    /// This slice is used by programs to prepare the accounts to
    /// be passed to a CPI.
    pub fn accounts_mut(&mut self) -> &mut [Account] {
        unsafe { self.accounts.as_mut_slice() }
    }

    /// Return the instruction data passed to the CPI being invoked.
    pub fn instruction_data(&self) -> &[u8] {
        self.instruction_data.as_slice()
    }

    /// Return a mutable instruction data slice for a CPI.
    ///
    /// This slice is used by programs to prepare the instruction data to
    /// be passed to a CPI.
    pub fn instruction_data_mut(&mut self) -> &mut [u8] {
        unsafe { self.instruction_data.as_mut_slice() }
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
    /// Return a reference to the runtime return-data scratchpad.
    pub fn borrow() -> Result<Ref<'static, ReturnData>, ProgramError> {
        unsafe {
            let borrow_state = memory::mut_ptr_at(HEAP_ADDRESS + RETURN_DATA_BORROW_FLAG_INDEX);

            if *borrow_state >= MAX_IMMUTABLE_BORROWS {
                return Err(ProgramError::InvalidArgument);
            }

            *borrow_state += 1;

            Ok(Ref {
                value: NonNull::from(Self::borrow_unchecked()),
                state: NonNull::new_unchecked(borrow_state),
                marker: PhantomData,
            })
        }
    }

    /// Return a reference to the runtime return-data scratchpad.
    ///
    /// # Safety
    ///
    /// This method does not update the borrow flag. The caller must ensure that
    /// no mutable borrow exists for the return-data scratchpad while the
    /// returned reference is used.
    pub unsafe fn borrow_unchecked() -> &'static ReturnData {
        unsafe { memory::ref_at::<ReturnData>(RETURN_DATA_ADDRESS) }
    }

    /// Return a mutable reference to the runtime return-data scratchpad.
    pub fn borrow_mut(len: usize) -> Result<RefMut<'static, ReturnData>, ProgramError> {
        unsafe {
            let borrow_state = memory::mut_ptr_at(HEAP_ADDRESS + RETURN_DATA_BORROW_FLAG_INDEX);

            if *borrow_state != NOT_BORROWED {
                return Err(ProgramError::InvalidArgument);
            }

            *borrow_state = MUTABLY_BORROWED;

            Ok(RefMut {
                value: NonNull::from(Self::borrow_mut_unchecked(len)),
                state: NonNull::new_unchecked(borrow_state),
                marker: PhantomData,
            })
        }
    }

    /// Return a mutable reference to the runtime return-data scratchpad.
    ///
    /// # Safety
    ///
    /// This method does not update the borrow flag. The caller must ensure that
    /// no other borrow exists for the return-data scratchpad while the returned
    /// mutable reference is used.
    pub unsafe fn borrow_mut_unchecked(len: usize) -> &'static mut ReturnData {
        let return_data = unsafe { memory::mut_ref_at::<ReturnData>(RETURN_DATA_ADDRESS) };

        set_buffer_length(return_data.data.ptr as u64, len as u64);

        return_data
    }

    /// Return the current return-data bytes.
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// Return the current return-data bytes.
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { self.data.as_mut_slice() }
    }

    /// Return `true` if the return-data scratchpad is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.data.len.get() == 0
    }

    /// Return the address of the program that last wrote return data.
    #[inline(always)]
    pub fn program(&self) -> Address {
        self.program.get()
    }

    /// Return `true` if the return data was written by `program`.
    #[inline(always)]
    pub fn written_by(&self, program: &Address) -> bool {
        let address_ptr = &raw const self.program as *const u64;
        let program_ptr = program.as_array().as_ptr() as *const u64;

        // SAFETY: Both pointers are valid for 32 bytes. `address_ptr` is aligned
        // to 8 bytes by the type layout, while `program_ptr` may be unaligned
        // because it points into a byte array.
        unsafe {
            read_volatile(address_ptr) == read_unaligned(program_ptr)
                && read_volatile(address_ptr.add(1)) == read_unaligned(program_ptr.add(1))
                && read_volatile(address_ptr.add(2)) == read_unaligned(program_ptr.add(2))
                && read_volatile(address_ptr.add(3)) == read_unaligned(program_ptr.add(3))
        }
    }
}

/// Represents a signer seed.
///
/// This struct contains the same information as a `[u8]`, but
/// has the memory layout as expected by `sol_invoke_signed_c`
/// syscall.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Seed<'bytes> {
    /// Seed bytes.
    pub(crate) seed: *const u8,

    /// Length of the seed bytes.
    pub(crate) len: u64,

    /// The pointer to the seed bytes is only valid while the `&'bytes [u8]` lives. Instead
    /// of holding a reference to the actual `[u8]`, which would increase the size of the
    /// type, we claim to hold a reference without actually holding one using a
    /// `PhantomData<&'bytes [u8]>`.
    _bytes: PhantomData<&'bytes [u8]>,
}

impl<'bytes> From<&'bytes [u8]> for Seed<'bytes> {
    fn from(value: &'bytes [u8]) -> Self {
        Self {
            seed: value.as_ptr(),
            len: value.len() as u64,
            _bytes: PhantomData::<&[u8]>,
        }
    }
}

impl<'bytes, const SIZE: usize> From<&'bytes [u8; SIZE]> for Seed<'bytes> {
    fn from(value: &'bytes [u8; SIZE]) -> Self {
        Self {
            seed: value.as_ptr(),
            len: value.len() as u64,
            _bytes: PhantomData::<&[u8]>,
        }
    }
}

impl Deref for Seed<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { from_raw_parts(self.seed, self.len as usize) }
    }
}

/// Represents a [program derived address][pda] (PDA) signer controlled by the
/// calling program.
///
/// [pda]: https://solana.com/docs/core/cpi#program-derived-addresses
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Signer<'bytes, 'seeds> {
    /// Signer seeds.
    pub(crate) seeds: *const Seed<'bytes>,

    /// Number of seeds.
    pub(crate) len: u64,

    /// The pointer to the seeds is only valid while the `&'seeds [Seed<'bytes>]` lives. Instead
    /// of holding a reference to the actual `[Seed<'bytes>]`, which would increase the size
    /// of the type, we claim to hold a reference without actually holding one using a
    /// `PhantomData<&'seeds [Seed<'bytes>]>`.
    _seeds: PhantomData<&'seeds [Seed<'bytes>]>,
}

impl<'bytes, 'seeds> From<&'seeds [Seed<'bytes>]> for Signer<'bytes, 'seeds> {
    fn from(value: &'seeds [Seed<'bytes>]) -> Self {
        Self {
            seeds: value.as_ptr(),
            len: value.len() as u64,
            _seeds: PhantomData::<&'seeds [Seed<'bytes>]>,
        }
    }
}

impl<'bytes, 'seeds, const SIZE: usize> From<&'seeds [Seed<'bytes>; SIZE]>
    for Signer<'bytes, 'seeds>
{
    fn from(value: &'seeds [Seed<'bytes>; SIZE]) -> Self {
        Self {
            seeds: value.as_ptr(),
            len: value.len() as u64,
            _seeds: PhantomData::<&'seeds [Seed<'bytes>]>,
        }
    }
}

/// Convenience macro for constructing a `[Seed; N]` array from a list of seeds
/// to create a [`Signer`].
///
/// # Example
///
/// Creating seeds array and signer for a PDA with a single seed and bump value:
/// ```
/// use solana_address::Address;
/// use abiv2::{seeds, cpi::{Signer}};
///
/// let pda_bump = 0xffu8;
/// let pda_ref = &[pda_bump];
/// let example_key = Address::default();
/// let seeds = seeds!(b"seed", example_key.as_ref(), pda_ref);
/// let signer = Signer::from(&seeds);
/// ```
#[macro_export]
macro_rules! seeds {
    ( $($seed:expr),* ) => {
        [$(
            $crate::cpi::Seed::from($seed),
        )*]
    };
}
