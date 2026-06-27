//! A library for creating ABIv2 Solana programs in Rust.

#![no_std]
#![allow(clippy::arithmetic_side_effects)]

use core::{
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr::{read_volatile, write_volatile, NonNull},
    slice::{from_raw_parts, from_raw_parts_mut},
};
// Re-export for downstream use:
//   - `solana_address`
//   - `solana_program_error`
pub use {
    solana_address::{self as address, Address},
    solana_program_error::{self as error, ProgramResult},
};

pub mod account;
pub mod context;
pub mod cpi;
pub mod entrypoint;
pub mod syscall;

/// Reserved bytes for account borrow flags.
pub const BORROW_FLAGS_SIZE: usize = 4096;

/// Address of the heap memory region.
///
/// This is the start address of the heap memory allocated by
/// the runtime. Note that the first [`BORROW_FLAGS_SIZE`] bytes
/// are reserved and should not be used by programs. Use
/// [`HEAP_START_ADDRESS`] for custom heap allocations.
pub const HEAP_ADDRESS: usize = 0x300000000;

/// Address of the heap region available to programs.
pub const HEAP_START_ADDRESS: usize = HEAP_ADDRESS + BORROW_FLAGS_SIZE;

/// Borrow-state value representing the maximum number of immutable borrows.
const MAX_IMMUTABLE_BORROWS: u8 = MUTABLY_BORROWED - 1;

/// Borrow-state sentinel used when account data is mutably borrowed.
const MUTABLY_BORROWED: u8 = u8::MAX;

/// Borrow-state value used when account data is not borrowed in any form,
/// immutably or mutably.
const NOT_BORROWED: u8 = 0;


/// A runtime-provided memory region.
///
/// `MemoryMapping` describes a contiguous region owned by the runtime. The
/// length is volatile because syscalls may update mapped metadata.
#[repr(C)]
pub(crate) struct MemoryMapping<T> {
    /// Address of the mapped region for `T`.
    ptr: *const T,

    /// Number of mapped `T` elements available.
    len: Volatile<u64>,
}

impl<T> MemoryMapping<T> {
    /// Return the mapped region as an immutable slice.
    #[inline(always)]
    pub(crate) fn as_slice(&self) -> &[T] {
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

/// Wrapper for values that must be accessed with volatile loads and stores.
///
/// This is used for fields backed by memory-mapped runtime state, where the
/// value may be updated by the runtime.
#[repr(transparent)]
pub struct Volatile<T: Copy>(T);

impl<T: Copy> Volatile<T> {
    /// Creates a volatile wrapper around a `T` value.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Reads the value with a volatile load.
    #[inline(always)]
    pub fn get(&self) -> T {
        // SAFETY: `self` guarantees that the wrapped field is valid for reads,
        // and `T: Copy` lets the value be returned by value.
        unsafe { read_volatile(&self.0) }
    }

    /// Writes `value` with a volatile store.
    #[inline(always)]
    pub fn set(&mut self, value: T) {
        // SAFETY: `&mut self` guarantees that the wrapped field is valid for
        // writes for the duration of the store.
        unsafe { write_volatile(&mut self.0, value) }
    }
}

/// An immutable reference to `T` with checked borrow rules.
///
/// The guard holds one immutable borrow and decrements the immutable borrow
/// count when it is dropped.
#[derive(Debug)]
pub struct Ref<'value, T: ?Sized> {
    /// Borrowed `T` value.
    value: NonNull<T>,

    /// Borrow-state byte for `T`.
    state: NonNull<u8>,

    /// Ties the raw `value` pointer to the lifetime of `T`.
    marker: PhantomData<&'value T>,
}

impl<'value, T: ?Sized> Ref<'value, T> {
    /// Maps this guard to a component of `T`.
    #[inline]
    pub fn map<U: ?Sized, F>(orig: Ref<'value, T>, f: F) -> Ref<'value, U>
    where
        F: FnOnce(&T) -> &U,
    {
        // Avoid decrementing the borrow flag on drop.
        let orig = ManuallyDrop::new(orig);
        Ref {
            value: NonNull::from(f(&*orig)),
            state: orig.state,
            marker: PhantomData,
        }
    }

    /// Tries to make a new `Ref` for a component of `T`.
    ///
    /// On failure, the original guard is returned alongside the error.
    #[inline]
    pub fn try_map<U: ?Sized, E>(
        orig: Ref<'value, T>,
        f: impl FnOnce(&T) -> Result<&U, E>,
    ) -> Result<Ref<'value, U>, (Self, E)> {
        // Avoid decrementing the borrow flag on drop.
        let orig = ManuallyDrop::new(orig);
        match f(&*orig) {
            Ok(value) => Ok(Ref {
                value: NonNull::from(value),
                state: orig.state,
                marker: PhantomData,
            }),
            Err(e) => Err((ManuallyDrop::into_inner(orig), e)),
        }
    }

    /// Maps this guard to a component of `T` if the closure returns one.
    ///
    /// On failure, the original guard is returned.
    #[inline]
    pub fn filter_map<U: ?Sized, F>(orig: Ref<'value, T>, f: F) -> Result<Ref<'value, U>, Self>
    where
        F: FnOnce(&T) -> Option<&U>,
    {
        // Avoid decrementing the borrow flag on drop.
        let orig = ManuallyDrop::new(orig);

        match f(&*orig) {
            Some(value) => Ok(Ref {
                value: NonNull::from(value),
                state: orig.state,
                marker: PhantomData,
            }),
            None => Err(ManuallyDrop::into_inner(orig)),
        }
    }
}

impl<T: ?Sized> Deref for Ref<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { self.value.as_ref() }
    }
}

impl<T: ?Sized> Drop for Ref<'_, T> {
    fn drop(&mut self) {
        // Decrement the immutable borrow count.
        unsafe { *self.state.as_mut() -= 1 };
    }
}

/// A mutable reference to `T` with checked borrow rules.
///
/// The guard holds the mutable borrow sentinel and clears the borrow state when
/// it is dropped.
#[derive(Debug)]
pub struct RefMut<'value, T: ?Sized> {
    /// Borrowed `T` value.
    value: NonNull<T>,

    /// Borrow-state byte for `T`.
    state: NonNull<u8>,

    /// Ties the raw `value` pointer to the lifetime of `T`.
    marker: PhantomData<&'value mut T>,
}

impl<'value, T: ?Sized> RefMut<'value, T> {
    /// Creates a mutable borrow guard from raw borrow-state parts.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `value` is the only active reference to the
    /// borrowed value and that `state` points to the borrow-state byte that
    /// should be cleared when the guard is dropped.
    #[doc(hidden)]
    #[inline(always)]
    pub unsafe fn new_unchecked(value: &'value mut T, state: *mut u8) -> Self {
        Self {
            value: NonNull::from(value),
            state: unsafe { NonNull::new_unchecked(state) },
            marker: PhantomData,
        }
    }

    /// Maps this guard to a component of `T`.
    #[inline]
    pub fn map<U: ?Sized, F>(orig: RefMut<'value, T>, f: F) -> RefMut<'value, U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        // Avoid clearing the borrow state on drop.
        let mut orig = ManuallyDrop::new(orig);
        RefMut {
            value: NonNull::from(f(&mut *orig)),
            state: orig.state,
            marker: PhantomData,
        }
    }

    /// Tries to make a new `RefMut` for a component of `T`.
    ///
    /// On failure, the original guard is returned alongside the error.
    #[inline]
    pub fn try_map<U: ?Sized, E>(
        orig: RefMut<'value, T>,
        f: impl FnOnce(&mut T) -> Result<&mut U, E>,
    ) -> Result<RefMut<'value, U>, (Self, E)> {
        // Avoid clearing the borrow state on drop.
        let mut orig = ManuallyDrop::new(orig);
        match f(&mut *orig) {
            Ok(value) => Ok(RefMut {
                value: NonNull::from(value),
                state: orig.state,
                marker: PhantomData,
            }),
            Err(e) => Err((ManuallyDrop::into_inner(orig), e)),
        }
    }

    /// Maps this guard to a component of `T` if the closure returns one.
    ///
    /// On failure, the original guard is returned.
    #[inline]
    pub fn filter_map<U: ?Sized, F>(
        orig: RefMut<'value, T>,
        f: F,
    ) -> Result<RefMut<'value, U>, Self>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        // Avoid clearing the borrow state on drop.
        let mut orig = ManuallyDrop::new(orig);
        match f(&mut *orig) {
            Some(value) => Ok(RefMut {
                value: NonNull::from(value),
                state: orig.state,
                marker: PhantomData,
            }),
            None => Err(ManuallyDrop::into_inner(orig)),
        }
    }
}

impl<T: ?Sized> Deref for RefMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { self.value.as_ref() }
    }
}
impl<T: ?Sized> DerefMut for RefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        unsafe { self.value.as_mut() }
    }
}

impl<T: ?Sized> Drop for RefMut<'_, T> {
    fn drop(&mut self) {
        // Clear the mutable-borrow sentinel.
        unsafe { *self.state.as_mut() = NOT_BORROWED };
    }
}

/// Module with functions to provide hints to the compiler about how code
/// should be optimized.
pub mod hint {
    /// A "dummy" function with a hint to the compiler that it is unlikely to be
    /// called.
    ///
    /// This function is used as a hint to the compiler to optimize other code
    /// paths instead of the one where the function is used.
    #[cold]
    pub const fn cold_path() {}

    /// Return the given `bool` value with a hint to the compiler that `true` is
    /// the likely case.
    #[inline(always)]
    pub const fn likely(b: bool) -> bool {
        if b {
            true
        } else {
            cold_path();
            false
        }
    }

    /// Return a given `bool` value with a hint to the compiler that `false` is
    /// the likely case.
    #[inline(always)]
    pub const fn unlikely(b: bool) -> bool {
        if b {
            cold_path();
            true
        } else {
            false
        }
    }
}
