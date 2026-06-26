use core::{marker::PhantomData, ops::Deref, ptr::NonNull, slice::from_raw_parts};

use solana_address::Address;
use solana_program_error::ProgramError;

use crate::{
    account::Account, context::TRANSACTION_METADATA_ADDRESS, syscall::set_buffer_length,
    MemoryMapping, RefMut, Volatile, HEAP_ADDRESS, MUTABLY_BORROWED, NOT_BORROWED,
};

/// Address of the runtime-managed CPI invoke parameters scratch pad.
///
/// Programs have a read-only access to this region, unless the `set_buffer_length`
/// syscall is used to specify its length and gain mutable access. This region
/// is used to write the parameters for a cross-program invocation.
pub(crate) const CPI_PARAMETERS_ADDRESS: usize = TRANSACTION_METADATA_ADDRESS + 0x30usize;

/// Index of the borrow flag for the CPI invoke paramters.
const BORROW_FLAG_INDEX: usize = 4088;

/// Runtime scratch area for CPI invocation parameters.
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
    /// Return a pointer to this account's borrow-state byte.
    ///
    /// # Safety
    ///
    /// The returned pointer may only be dereferenced when the runtime has
    /// reserved account borrow flags.
    #[inline(always)]
    unsafe fn borrow_state() -> *mut u8 {
        (HEAP_ADDRESS + BORROW_FLAG_INDEX) as *mut u8
    }

    pub fn for_invocation(
        accounts_len: usize,
        data_len: usize,
    ) -> Result<RefMut<'static, Self>, ProgramError> {
        unsafe {
            let borrow_state = Self::borrow_state();

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

    /// Returns the runtime scratch parameters for a cross-program invocation.
    ///
    /// This resizes the instructiondata and accounts scratch pads and
    /// returns a mutable reference to the fixed runtime-managed parameters region.
    ///
    /// # Safety
    ///
    /// The caller must ensure that no other reference to the parameters region is
    /// alive while the returned mutable reference is used. Calling this function
    /// again before the returned reference is no longer used can create aliasing
    /// mutable references and cause undefined behavior.
    pub unsafe fn for_invocation_unchecked(
        accounts_len: usize,
        data_len: usize,
    ) -> &'static mut Parameters {
        let parameters = unsafe { &mut *(CPI_PARAMETERS_ADDRESS as *mut Parameters) };

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
