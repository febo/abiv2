mod allocator;

pub use allocator::BumpAllocator;
use {
    crate::{account::Account, context::InstructionContext},
    solana_program_error::{ProgramError, ProgramResult},
};

pub const SUCCESS: u64 = 0;

#[macro_export]
macro_rules! entrypoint {
    ( $process_instruction:expr ) => {
        $crate::program_entrypoint!($process_instruction);
        $crate::default_allocator!();
        $crate::default_panic_handler!();
    };
}

#[macro_export]
macro_rules! program_entrypoint {
    ( $process_instruction:expr ) => {
        /// Program entrypoint.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn entrypoint(
            instruction: *const $crate::context::InstructionContext,
            accounts: *const $crate::account::Account,
            accounts_len: u64,
            instruction_data: *const u8,
            instruction_data_len: u64,
        ) -> u64 {
            unsafe {
                $crate::entrypoint::process_entrypoint(
                    &*instruction,
                    core::slice::from_raw_parts(accounts, accounts_len as usize),
                    core::slice::from_raw_parts(instruction_data, instruction_data_len as usize),
                    $process_instruction,
                )
            }
        }
    };
}

/// Process the entrypoint for a program.
///
/// # Safety
///
/// The caller must ensure that all inputs represent valid program input
/// parameters serialized by the SVM loader. Additionally, they should last for
/// the lifetime of the program execution.
#[inline(always)]
pub unsafe fn process_entrypoint<F>(
    context: &InstructionContext,
    accounts: &[Account],
    instruction_data: &[u8],
    process_instruction: F,
) -> u64
where
    F: FnOnce(&InstructionContext, &[Account], &[u8]) -> ProgramResult,
{
    match process_instruction(context, accounts, instruction_data) {
        Ok(()) => SUCCESS,
        Err(e) => program_error_to_u64(e),
    }
}

/// This function is marked as `#[cold]` to move the error conversion from the
/// "hot path" of the entrypoint.
#[inline(never)]
fn program_error_to_u64(error: ProgramError) -> u64 {
    error.into()
}

/// Default global allocator.
///
/// This macro combines a static memory allocator with a bump allocator. It
/// takes a list of pointer-name and type pairs, allocates them in the heap
/// memory region at compile time, and provides accessors for them during
/// program execution. Any remaining heap space is made available to the bump
/// allocator.
///
/// The statically allocated values are defined in the `default_allocator!`
/// macro definition:
///
/// ```ignore
/// default_allocator!({
///    buffer: [u8; 5000],
///    counter: u64,
///    some_other_allocation: u32,
/// });
/// ```
///
/// This makes checked accessors available under an `alloc` module:
///   - `fn buffer() -> Result<RefMut<[u8; 5000]>, ProgramError>`
///   - `fn counter() -> Result<RefMut<u64>, ProgramError>`
///   - `fn some_other_allocation() -> Result<RefMut<u32>, ProgramError>`
///
/// The allocated values can be directly used in a program:
/// ```ignore
/// let buffer = crate::alloc::buffer()?;
/// let counter = crate::alloc::counter()?;
/// let some_other_allocation = crate::alloc::some_other_allocation()?;
/// ```
///
/// If the static allocation exceeds the maximum allowed heap space, a
/// compile-time error is generated.
#[macro_export]
macro_rules! default_allocator {
    () => {
        $crate::default_allocator!({});
    };
    ( { $( $ptr_name:ident : $ptr_ty:ty ),* $(,)? } ) => {
        pub(crate) mod alloc {
            //! Static memory allocations for the program.

            /// Maximum heap length in bytes that a program can request.
            const MAX_HEAP_LENGTH: usize = 256 * 1024;

            /// The `StaticAllocator` struct defines the static memory locations
            /// for the program and the remaining heap size.
            struct StaticAllocator {
                start: usize,
                len: usize,
                $(
                    $ptr_name: (u64, u64)
                ),*
            }

            impl StaticAllocator {
                #[allow(clippy::arithmetic_side_effects)]
                const fn new() -> Self {
                    // Static allocations start at the beginning of the heap.
                    let mut start: usize = $crate::HEAP_START_ADDRESS;

                    $(
                        let align = core::mem::align_of::<u64>();
                        // Align the start address to the next multiple of `align`.
                        let boundary = (start + align - 1) & !(align - 1);
                        let $ptr_name = (
                            boundary as u64,
                            boundary.saturating_add(core::mem::size_of::<u64>()) as u64
                        );

                        start = boundary
                            .saturating_add(core::mem::size_of::<u64>())
                            .saturating_add(core::mem::size_of::<$ptr_ty>());
                    )*

                    // Align the start address to the next multiple of `u64`.
                    let align = core::mem::align_of::<u64>();
                    let start = (start + align - 1) & !(align - 1);
                    // Calculate the length of the allocated memory.
                    let len = start - $crate::HEAP_START_ADDRESS;

                    Self {
                        start,
                        // A compile-time check to ensure that the length of the allocated memory
                        // does not exceed the heap size.
                        len: if len > MAX_HEAP_LENGTH {
                            panic!("Heap allocation overflow: \
                                The size of the allocated memory exceeds the available heap space.");
                            } else {
                                MAX_HEAP_LENGTH - len
                            },
                        $(
                            $ptr_name,
                        )*
                    }
                }
            }

            // A static instance of the allocator.
            const STATIC_ALLOCATOR: StaticAllocator = StaticAllocator::new();

            /// Unchecked accessors for static allocations.
            pub mod unchecked {
                $(
                    #[doc = concat!("Return an unchecked mutable reference to `", stringify!($ptr_name), "`.")]
                    ///
                    /// # Safety
                    ///
                    /// This method does not return a `RefMut`, leaving the
                    /// borrow flag untouched. The caller must guarantee that
                    /// Rust aliasing rules are followed and that multiple
                    /// mutable references are not created at the same time.
                    #[inline(always)]
                    pub unsafe fn $ptr_name() -> &'static mut $ptr_ty {
                        // SAFETY: The pointer is within a valid range and
                        // aligned to `T`.
                        unsafe { &mut *(super::STATIC_ALLOCATOR.$ptr_name.1 as *mut $ptr_ty) }
                    }
                )*
            }

            $(
                #[doc = concat!("Return a checked mutable reference to `", stringify!($ptr_name), "`.")]
                #[inline(always)]
                pub fn $ptr_name() -> Result<$crate::RefMut<'static, $ptr_ty>, $crate::error::ProgramError> {
                    let borrow_state = STATIC_ALLOCATOR.$ptr_name.0 as *mut u8;

                    if unsafe { *borrow_state != 0 } {
                       return Err($crate::error::ProgramError::InvalidArgument);
                    }

                    unsafe { *borrow_state = u8::MAX };

                    // SAFETY: The borrow flag is marked as mutably borrowed for the lifetime
                    // of the returned guard.
                    unsafe {
                        Ok($crate::RefMut::new_unchecked(
                            unchecked::$ptr_name(),
                            borrow_state,
                        ))
                    }
                }
            )*

            #[cfg(any(target_os = "solana", target_arch = "bpf"))]
            #[global_allocator]
            static A: $crate::entrypoint::BumpAllocator = unsafe {
                $crate::entrypoint::BumpAllocator::new_unchecked(
                    STATIC_ALLOCATOR.start,
                    STATIC_ALLOCATOR.len,
                )
            };

            /// A default allocator for when the program is compiled for a target other than
            /// `"solana"` or `"bpf"`.
            ///
            /// This links the `std` library, which will set up a default global allocator.
            #[cfg(not(any(target_os = "solana", target_arch = "bpf")))]
            mod __private {
                extern crate std as __std;
            }
        }
    };
}

/// Default panic hook.
///
/// This macro sets up a default panic hook that logs the file where the panic
/// occurred. It acts as a hook after Rust runtime panics; syscall `abort()`
/// will be called after it returns.
#[cfg(feature = "std")]
#[macro_export]
macro_rules! default_panic_handler {
    () => {
        // Make sure the "std" is present.
        extern crate std as __std;
        /// Default panic handler.
        #[cfg(any(target_os = "solana", target_arch = "bpf"))]
        #[no_mangle]
        fn custom_panic(info: &core::panic::PanicInfo<'_>) {
            if let Some(location) = info.location() {
                let location = location.file();
                unsafe { $crate::syscall::sol_log_(location.as_ptr(), location.len() as u64) };
            }
            // Panic reporting.
            const PANICKED: &str = "** PANICKED **";
            unsafe { $crate::syscall::sol_log_(PANICKED.as_ptr(), PANICKED.len() as u64) };
        }
    };
}

/// A global `#[panic_handler]` for `no_std` programs.
///
/// This macro sets up a default panic handler that logs the location (file,
/// line and column) where the panic occurred and then calls the syscall
/// `abort()`.
///
/// This macro should be used when all crates are `no_std`.
#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! default_panic_handler {
    () => {
        /// A panic handler for `no_std`.
        #[cfg(any(target_os = "solana", target_arch = "bpf"))]
        #[panic_handler]
        fn handler(info: &core::panic::PanicInfo<'_>) -> ! {
            if let Some(location) = info.location() {
                unsafe {
                    $crate::syscall::sol_panic_(
                        location.file().as_ptr(),
                        location.file().len() as u64,
                        location.line() as u64,
                        location.column() as u64,
                    )
                }
            } else {
                // Panic reporting.
                const PANICKED: &str = "** PANICKED **";
                unsafe {
                    $crate::syscall::sol_log_(PANICKED.as_ptr(), PANICKED.len() as u64);
                    $crate::syscall::abort();
                }
            }
        }

        /// A panic handler for when the program is compiled on a target different than
        /// `"solana"`.
        ///
        /// This links the `std` library, which will set up a default panic handler.
        #[cfg(not(any(target_os = "solana", target_arch = "bpf")))]
        mod __private_panic_handler {
            extern crate std as __std;
        }
    };
}
