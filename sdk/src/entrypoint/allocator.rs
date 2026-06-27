use {
    crate::hint::unlikely,
    core::{
        alloc::{GlobalAlloc, Layout},
        mem::size_of,
        ptr::null_mut,
    },
};

/// Maximum heap length in bytes that a program can request.
const MAX_HEAP_LENGTH: u32 = 256 * 1024;

/// The bump allocator used as the default Rust heap when running programs.
///
/// The allocator uses a forward bump allocation strategy, where memory is
/// allocated by moving a pointer forward in a pre-allocated memory
/// region. The current position of the heap pointer is stored at the
/// start of the memory region.
///
/// This implementation relies on the runtime to zero out memory and to
/// enforce the limit of the heap memory region. Use of memory outside
/// the allocated region will result in a runtime error.
#[derive(Clone, Debug)]
pub struct BumpAllocator {
    start: usize,
    end: usize,
}

impl BumpAllocator {
    /// Creates the allocator tied to specific range of addresses.
    ///
    /// # Safety
    ///
    /// This is unsafe in most situations, unless you are totally sure that
    /// the provided start address and length can be written to by the
    /// allocator, and that the memory will be usable for the lifespan of
    /// the allocator. The start address must be aligned to `usize` and
    /// the length must be at least `size_of::<usize>()` bytes.
    ///
    /// For Solana on-chain programs, a certain address range is reserved,
    /// so the allocator can be given those addresses. In general,
    /// the `len` is set to the maximum heap length allowed by the
    /// runtime. The runtime will enforce the actual heap size
    /// requested by the program.
    pub const unsafe fn new_unchecked(start: usize, len: usize) -> Self {
        Self {
            start,
            end: start + len,
        }
    }
}

// Integer arithmetic in this global allocator implementation is safe when
// operating on the prescribed `BumpAllocator::start` and
// `BumpAllocator::end`. Any other use may overflow and is thus unsupported
// and at one's own risk.
#[allow(clippy::arithmetic_side_effects)]
unsafe impl GlobalAlloc for BumpAllocator {
    /// Allocates memory as described by the given `layout` using a forward
    /// bump allocator.
    ///
    /// Return a pointer to newly-allocated memory, or `null` to indicate
    /// allocation failure.
    ///
    /// # Safety
    ///
    /// `layout` must have non-zero size. Attempting to allocate for a
    /// zero-sized layout will result in undefined behaviour.
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Reads the current position of the heap pointer.
        //
        // Integer-to-pointer cast: the caller guarantees that `self.start` is a valid
        // address for the lifetime of the allocator and aligned to `usize`.
        let pos_ptr = self.start as *mut usize;
        let mut pos = unsafe { *pos_ptr };

        if unlikely(pos == 0) {
            // First time, set starting position.
            pos = self.start + size_of::<usize>();
        }

        // Determines the allocation address, adjusting the alignment for the
        // type being allocated.
        let allocation = (pos + layout.align() - 1) & !(layout.align() - 1);

        if unlikely(layout.size() > MAX_HEAP_LENGTH as usize)
            || unlikely(self.end < allocation + layout.size())
        {
            return null_mut();
        }

        // Updates the heap pointer.
        unsafe { *pos_ptr = allocation + layout.size() };

        allocation as *mut u8
    }

    /// Behaves like `alloc`, but also ensures that the contents are set to
    /// zero before being returned.
    ///
    /// This method relies on the runtime to zero out the memory when
    /// reserving the heap region, so it simply calls `alloc`.
    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { self.alloc(layout) }
    }

    /// This method has no effect since the bump allocator does not free
    /// memory.
    #[inline]
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}

/// An allocator that denies dynamic memory allocations.
#[derive(Clone, Debug)]
pub struct DenyAllocator;

unsafe impl GlobalAlloc for DenyAllocator {
    #[inline]
    unsafe fn alloc(&self, _: Layout) -> *mut u8 {
        panic!("** DenyAllocator::alloc() does not allocate memory **");
    }

    #[inline]
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        // Allocations are denied, so there is nothing to free.
    }
}
