use crate::memory::{self, TRANSACTION_ACCOUNTS_PAYLOAD_ADDRESS};

pub(crate) const unsafe fn get<T>(index: usize) -> &'static T {
    // SAFETY: The runtime maps sysvar data as transaction account payload at
    // [`TRANSACTION_ACCOUNTS_PAYLOAD_ADDRESS`].
    unsafe {
        memory::ref_at(memory::mapping_ptr(
            TRANSACTION_ACCOUNTS_PAYLOAD_ADDRESS,
            index,
        ))
    }
}

macro_rules! impl_get {
    ( $sysvar:ty, $index:literal ) => {
        impl $sysvar {
            /// Return a pointer to this sysvar's borrow-state byte.
            ///
            /// # Safety
            ///
            /// The returned pointer may only be dereferenced when the runtime has
            /// reserved account borrow flags.
            #[inline(always)]
            unsafe fn borrow_state() -> *mut u8 {
                $crate::memory::mut_ptr_at(
                    $crate::memory::HEAP_ADDRESS
                        + ($crate::memory::MAX_TRANSACTION_ACCOUNTS - $index),
                )
            }

            #[doc = concat!("Return a reference to the `", stringify!($ssyvar), "` sysvar.")]
            #[inline(always)]
            pub fn get() -> Result<$crate::Ref<'static, Self>, $crate::error::ProgramError> {
                let borrow_state = unsafe { Self::borrow_state() };

                if unsafe { *borrow_state } >= $crate::MAX_IMMUTABLE_BORROWS {
                    return Err($crate::error::ProgramError::AccountBorrowFailed);
                }

                // SAFETY: The `borrow_state` is a mutable pointer to the borrow state,
                // which is guaranteed to be valid.
                unsafe { *borrow_state += 1 };

                Ok($crate::Ref {
                    // SAFETY: The borrow flag was updated.
                    value: core::ptr::NonNull::from(unsafe { Self::get_unchecked() }),
                    state: unsafe { core::ptr::NonNull::new_unchecked(borrow_state) },
                    marker: core::marker::PhantomData,
                })
            }

            #[doc = concat!("Return a reference to the `", stringify!($ssyvar), "` sysvar.")]
            ///
            /// # Safety
            ///
            /// This method does not update the borrow flag. The caller must ensure that
            /// no other borrow exists for the sysvar data for the duration of the
            /// returned reference.
            #[inline(always)]
            pub const unsafe fn get_unchecked() -> &'static Self {
                unsafe { $crate::sysvar::get($crate::memory::MAX_TRANSACTION_ACCOUNTS - $index) }
            }
        }
    };
}

pub mod clock;
pub mod rent;
