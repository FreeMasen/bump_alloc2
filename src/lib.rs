#![cfg_attr(feature = "nightly", feature(allocator_api))]

use std::alloc::{GlobalAlloc, Layout, handle_alloc_error};
use std::cell::UnsafeCell;
use std::ptr::{NonNull, null_mut};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[cfg(not(feature = "nightly"))]
use allocator_api2::alloc::{AllocError, Allocator};
#[cfg(feature = "nightly")]
use std::alloc::{AllocError, Allocator};

fn align_to(size: usize, align: usize) -> usize {
    (size + align - 1) & !(align - 1)
}

struct Inner {
    offset: AtomicUsize,
    mmap: *mut u8,
    initializing: AtomicBool,
}

pub struct BumpAlloc {
    inner: UnsafeCell<Inner>,
    size: usize,
}

impl Default for BumpAlloc {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Sync for BumpAlloc {}

impl BumpAlloc {
    /// Create a new instance of the bump allocator with a default initial size of 1 gigabyte
    pub const fn new() -> BumpAlloc {
        BumpAlloc::with_size(1024 * 1024 * 1024)
    }

    /// Create a new instance of the bump allocator with the provided size
    pub const fn with_size(size: usize) -> BumpAlloc {
        BumpAlloc {
            inner: UnsafeCell::new(Inner {
                initializing: AtomicBool::new(true),
                mmap: null_mut(),
                offset: AtomicUsize::new(0),
            }),
            size,
        }
    }
}

// type of the size parameter to VirtualAlloc
#[cfg(all(windows, target_pointer_width = "32"))]
type WindowsSize = u32;
#[cfg(all(windows, target_pointer_width = "64"))]
type WindowsSize = u64;

#[cfg(windows)]
unsafe fn mmap_wrapper(size: usize) -> *mut u8 {
    kernel32::VirtualAlloc(
        null_mut(),
        size as WindowsSize,
        winapi::um::winnt::MEM_COMMIT | winapi::um::winnt::MEM_RESERVE,
        winapi::um::winnt::PAGE_READWRITE,
    ) as *mut u8
}

#[cfg(all(unix, not(target_os = "android")))]
unsafe fn mmap_wrapper(size: usize) -> *mut u8 {
    unsafe {
        libc::mmap(
            null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        ) as *mut u8
    }
}

unsafe impl GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        Allocator::allocate(&self, layout)
            .map(|v| v.as_ptr().cast())
            .unwrap_or_else(|_| null_mut())
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

unsafe impl Allocator for BumpAlloc {
    fn allocate(&self, layout: Layout) -> Result<std::ptr::NonNull<[u8]>, AllocError> {
        unsafe {
            let inner = &mut *self.inner.get();

            // If initializing is true it means we need to do the original mmap.
            if inner.initializing.swap(false, Ordering::Relaxed) {
                inner.mmap = mmap_wrapper(self.size);

                if (*inner.mmap as isize) == -1isize {
                    handle_alloc_error(layout);
                }
            } else {
                // Spin loop waiting on the mmap to be ready.
                while 0 == inner.offset.load(Ordering::Relaxed) {}
            }

            let bytes_required = align_to(layout.size() + layout.align(), layout.align());

            let my_offset = inner.offset.fetch_add(bytes_required, Ordering::Relaxed);

            let aligned_offset = align_to(my_offset, layout.align());

            if (aligned_offset + layout.size()) > self.size {
                handle_alloc_error(layout);
            }

            let ret_ptr = inner.mmap.add(aligned_offset);
            let nn = NonNull::new(ret_ptr).ok_or(AllocError)?;
            Ok(NonNull::slice_from_raw_parts(nn, layout.size()))
        }
    }

    unsafe fn deallocate(&self, _ptr: std::ptr::NonNull<u8>, _layout: Layout) {}
}
