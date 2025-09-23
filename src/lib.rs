#![cfg_attr(feature = "nightly", feature(allocator_api))]
#![doc = include_str!("../README.md")]

use std::{
    alloc::{GlobalAlloc, Layout, handle_alloc_error},
    ptr::{NonNull, null_mut},
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

/// For unix systems, mmap will return !1usize on failure
#[cfg(not(windows))]
use libc::MAP_FAILED;

/// For windows systems, mmap will return null on failure
#[cfg(windows)]
pub const MAP_FAILED: *mut u8 = null_mut();

#[cfg(not(feature = "nightly"))]
use allocator_api2::alloc::{AllocError, Allocator};
#[cfg(feature = "nightly")]
use std::alloc::{AllocError, Allocator};

pub struct BumpAlloc {
    ptr: AtomicPtr<u8>,
    remaining: AtomicUsize,
    #[cfg(feature = "allocated")]
    size: AtomicUsize,
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
            ptr: AtomicPtr::new(null_mut()),
            remaining: AtomicUsize::new(size),
            #[cfg(feature = "allocated")]
            size: AtomicUsize::new(size),
        }
    }

    /// get the allocated
    #[cfg(feature = "allocated")]
    pub fn allocated(&self) -> usize {
        let sz = self.size.load(Ordering::Relaxed);
        let rm = self.remaining.load(Ordering::Relaxed);
        sz.wrapping_sub(rm)
    }

    /// Get the number of bytes remaining
    pub fn remaining(&self) -> usize {
        self.remaining.load(Ordering::Relaxed)
    }

    fn ensure_init(&self) -> Result<(), AllocError> {
        self.ptr
            .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |p| {
                if !p.is_null() {
                    return Some(p);
                }
                unsafe {
                    let new_ptr = mmap_wrapper(self.remaining.load(Ordering::Relaxed));
                    if new_ptr.cast() == MAP_FAILED {
                        return None;
                    }
                    Some(new_ptr)
                }
            })
            .map_err(|_| AllocError)
            .map(|_| ())
    }

    fn bump(&self, size: usize, align: usize) -> Result<usize, AllocError> {
        let align_mask_to_round_down = !(align - 1);
        let mut allocated = 0;
        self.remaining
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |mut remaining| {
                if size > remaining {
                    return None;
                }
                remaining -= size;
                remaining &= align_mask_to_round_down;
                allocated = remaining;
                Some(remaining)
            })
            .map_err(|_| AllocError)?;
        Ok(allocated)
    }

    fn get_ptr(&self, offset: usize) -> *mut u8 {
        unsafe { self.ptr.load(Ordering::Release).add(offset) }
    }
}

// type of the size parameter to VirtualAlloc
#[cfg(all(windows, target_pointer_width = "32"))]
type WindowsSize = u32;
#[cfg(all(windows, target_pointer_width = "64"))]
type WindowsSize = u64;

#[cfg(windows)]
unsafe fn mmap_wrapper(size: usize) -> *mut u8 {
    unsafe {
        kernel32::VirtualAlloc(
            null_mut(),
            size as WindowsSize,
            winapi::um::winnt::MEM_COMMIT | winapi::um::winnt::MEM_RESERVE,
            winapi::um::winnt::PAGE_READWRITE,
        ) as *mut u8
    }
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
        let Ok(ptr) = Allocator::allocate(&self, layout).map(|v| v.as_ptr().cast()) else {
            handle_alloc_error(layout)
        };
        ptr
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

unsafe impl Allocator for BumpAlloc {
    fn allocate(&self, layout: Layout) -> Result<std::ptr::NonNull<[u8]>, AllocError> {
        self.ensure_init()?;
        let allocated = self.bump(layout.size(), layout.align())?;
        let ret_ptr = self.get_ptr(allocated);
        let nn = NonNull::new(ret_ptr).ok_or(AllocError)?;
        Ok(NonNull::slice_from_raw_parts(nn, layout.size()))
    }

    unsafe fn deallocate(&self, _ptr: std::ptr::NonNull<u8>, _layout: Layout) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_u32_state_methods() {
        let u = u32::MAX;
        let layout = Layout::for_value(&u);
        let bump = BumpAlloc::with_size(layout.size());
        unsafe {
            let ptr = bump.alloc(layout).cast::<u32>();
            ptr.write(u);
            assert_eq!(Some(&u), ptr.as_ref())
        }
        #[cfg(feature = "allocated")]
        assert_eq!(bump.allocated(), layout.size());
        assert_eq!(bump.remaining(), 0);
    }
}
