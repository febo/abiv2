//! Runtime memory-map definitions and raw address helpers.
//!
//! ABIv2 programs run with several runtime-owned regions mapped at fixed
//! virtual addresses. Transaction metadata, instruction metadata, transaction
//! accounts, CPI parameters, return data, and borrow flags are all read
//! from those regions by casting the known address to the expected ABI type.
//!
//! This module centralizes the memory map and the small set of raw helpers used
//! to turn those addresses into Rust references. The helpers are unsafe because
//! the caller must know that the runtime mapped a valid value of the requested
//! type at the provided address, with suitable alignment and lifetime.

use {
    crate::Volatile,
    core::{
        mem::size_of,
        slice::{from_raw_parts, from_raw_parts_mut},
    },
};

/// Reserved bytes for borrow flags.
///
/// Safe API for account borrows and scratchpads use a byte flag to
/// track borrows. These are stored in the first `4096` bytes of the
/// heap region. When defining a custom allocator, the [`HEAP_START_ADDRESS`]
/// should be used as the start address for allocations.
///
/// A program without heap space can still use `unchecked` variants
/// to bypass the borrow tracking.
pub const BORROW_FLAGS_SIZE: usize = 4096;

/// Maximum number of accounts (including sysvars) in a transaction.
pub const MAX_TRANSACTION_ACCOUNTS: usize = 4088;

/// Address of the heap memory region.
///
/// This is the start address of the heap memory allocated by the runtime. The
/// first [`BORROW_FLAGS_SIZE`] bytes are reserved for borrow flags and should
/// not be directly used by programs. Use [`HEAP_START_ADDRESS`] for custom heap
/// allocations.
pub const HEAP_ADDRESS: usize = 0x300000000;

/// Address of the heap region available to programs.
pub const HEAP_START_ADDRESS: usize = HEAP_ADDRESS + BORROW_FLAGS_SIZE;

/// Address of the runtime-managed transaction metadata memory region.
///
/// The transaction metadata is divided into three components:
///
///   - `ReturnData`: scratchpad available to programs to read/write
///     return data. This is specified by [`RETURN_DATA_ADDRESS`].
///
///   - `Parameters`: scratchpad available to pass accounts and instruction
///     data to cross-program invocations. This is specified by
///     [`CPI_PARAMETERS_ADDRESS`].
///
///   - `TransactionContext`: metadata information about the current
///     executing transaction. This is specofied as [`TRANSACTION_CONTEXT_ADDRESS`].
const TRANSACTION_METADATA_ADDRESS: usize = 0x400000000usize;

/// Address of the runtime-managed transaction accounts memory region.
pub(crate) const TRANSACTION_ACCOUNTS_ADDRESS: usize = 0x500000000usize;

/// Address of the runtime-managed instruction region.
pub(crate) const INSTRUCTION_ADDRESS: usize = 0x600000000usize;

/// Address of the runtime-managed transaction accounts payload memory region.
pub(crate) const TRANSACTION_ACCOUNTS_PAYLOAD_ADDRESS: usize = 0x800000000usize;

/// Address of the runtime-managed return data scratchpad.
pub(crate) const RETURN_DATA_ADDRESS: usize = TRANSACTION_METADATA_ADDRESS;

/// Address of the runtime-managed CPI invoke parameters scratchpad.
pub(crate) const CPI_PARAMETERS_ADDRESS: usize = TRANSACTION_METADATA_ADDRESS + 0x30usize;

/// Address of the transaction context data.
pub(crate) const TRANSACTION_CONTEXT_ADDRESS: usize = TRANSACTION_METADATA_ADDRESS + 0x50usize;

/// Length (in bytes) of a mapping page.
///
/// This represents the gap between account mappings.
pub(crate) const MAPPING_PAGE_LENGTH: usize = 0x100000000usize;

/// Return the address of `index` within a runtime array of `T`.
#[inline(always)]
pub(crate) const fn element_address<T>(base: usize, index: usize) -> usize {
    base + (index * size_of::<T>())
}

/// Return the address of `index` within a runtime memoyr mapping.
#[inline(always)]
pub(crate) const fn mapping_ptr(base: usize, index: usize) -> usize {
    base + (index * MAPPING_PAGE_LENGTH)
}

/// Return a mutable pointer for a fixed runtime address.
#[inline(always)]
pub(crate) const fn mut_ptr_at<T>(address: usize) -> *mut T {
    address as *mut T
}

/// Return a shared reference to a value at a fixed runtime address.
///
/// # Safety
///
/// The caller must guarantee that `address` points to a valid, properly
/// aligned `T` for the full program execution.
#[inline(always)]
pub(crate) const unsafe fn ref_at<T>(address: usize) -> &'static T {
    unsafe { &*(address as *const T) }
}

/// Return a mutable reference to a value at a fixed runtime address.
///
/// # Safety
///
/// The caller must guarantee that `address` points to a valid, properly
/// aligned `T` and that no other references alias it for the returned lifetime.
#[inline(always)]
pub(crate) unsafe fn mut_ref_at<T>(address: usize) -> &'static mut T {
    unsafe { &mut *(address as *mut T) }
}

/// Return a shared slice at a fixed runtime address.
///
/// # Safety
///
/// The caller must guarantee that `address` points to `len` valid,
/// properly-aligned `T` values for the full program execution.
#[inline(always)]
pub(crate) unsafe fn slice_at<T>(address: usize, len: usize) -> &'static [T] {
    unsafe { from_raw_parts(address as *const T, len) }
}

/// A runtime-provided memory region.
///
/// `MemoryMapping` describes a contiguous region owned by the runtime. The
/// length is volatile because syscalls may update mapped metadata.
#[repr(C)]
pub(crate) struct MemoryMapping<T> {
    /// Address of the mapped region for `T`.
    pub(crate) ptr: *const T,

    /// Number of mapped `T` elements available.
    pub(crate) len: Volatile<u64>,
}

impl<T> MemoryMapping<T> {
    /// Return the mapped region as an immutable slice.
    #[inline(always)]
    pub(crate) fn as_slice(&self) -> &[T] {
        // SAFETY: The runtime owns the pointer and length for this mapping.
        unsafe { from_raw_parts(self.ptr, self.len.get() as usize) }
    }

    /// Return a mutable slice for the memory region.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the mapped region is writable and that no
    /// other references alias the returned mutable slice.
    #[inline(always)]
    #[allow(clippy::mut_from_ref)]
    pub(crate) unsafe fn as_mut_slice(&self) -> &mut [T] {
        unsafe { from_raw_parts_mut(self.ptr as *mut T, self.len.get() as usize) }
    }
}
