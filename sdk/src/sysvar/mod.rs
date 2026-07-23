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
            #[doc = concat!("Return a reference to the `", stringify!($ssyvar), "` sysvar.")]
            #[inline(always)]
            pub const fn get() -> &'static Self {
                unsafe { $crate::sysvar::get($crate::memory::MAX_TRANSACTION_ACCOUNTS - $index) }
            }
        }
    };
}

pub mod clock;
pub mod rent;
