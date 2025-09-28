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
            size: AtomicUsize::new(size),
        }
    }

    /// get the allocated
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
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |p| {
                if !p.is_null() {
                    return Some(p);
                }
                unsafe {
                    let new_ptr = mmap_wrapper(self.remaining.load(Ordering::Relaxed));
                    debug_assert_ne!(
                        new_ptr.cast(),
                        MAP_FAILED,
                        "mmap failed: {:?}",
                        std::io::Error::last_os_error()
                    );
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
                    println!("would overallocate");
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
        unsafe { self.ptr.load(Ordering::Relaxed).add(offset) }
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

#[cfg(windows)]
unsafe fn mummap_wrapper(ptr: *mut u8, _size: usize) {
    unsafe { kernel32::VirtualFree(ptr, 0, winapi::um::winnt::MEM_RELEASE) };
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

#[cfg(all(unix, not(target_os = "android")))]
unsafe fn mummap_wrapper(addr: *mut u8, len: usize) {
    unsafe { libc::munmap(addr.cast(), len) };
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

impl Drop for BumpAlloc {
    fn drop(&mut self) {
        reset_alloc(self);
    }
}

fn reset_alloc(b: &BumpAlloc) {
    b.ptr
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |mut p| {
            if p.is_null() {
                return None;
            }
            let size = b.size.load(Ordering::Relaxed);
            unsafe {
                mummap_wrapper(p, size);
            }
            p = null_mut();
            Some(p)
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::*;

    static CONCURRENT_ITER: LazyLock<usize> = LazyLock::new(|| {
        std::env::var("BA2_CONCURRENT_ITERS")
            .map_err(|_| ())
            .and_then(|v| v.parse::<usize>().map_err(|_| ()))
            .unwrap_or(1000)
    });

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
        assert_eq!(bump.allocated(), layout.size());
        assert_eq!(bump.remaining(), 0);
    }

    fn concurrent_inner() {
        let a = Box::new(BumpAlloc::new());
        let a2 = Box::leak(a);
        // generate a thread callback that will allocate 64bits and return the numeric
        // value of the start pointer before allocation.
        fn gen_thread(a2: &'static BumpAlloc) -> impl FnOnce() -> usize {
            || {
                // load the current pointer
                let start = a2.ptr.load(Ordering::Relaxed).addr();
                // perform an allocation of 64 bits
                a2.allocate(Layout::for_value(&0u64)).unwrap();
                if start == 0 {
                    // if start was null, we assert that the start and the current
                    // address are not equal
                    assert_ne!(a2.ptr.load(Ordering::Relaxed).addr(), start)
                } else {
                    // if start was not-null, we assert that no other thread has
                    // clobbered the other allocation
                    assert_eq!(a2.ptr.load(Ordering::Relaxed).addr(), start)
                }
                // returning the start to the joiner
                start
            }
        }
        let th1 = shuttle::thread::Builder::new()
            .name("tread1".to_string())
            .spawn(gen_thread(a2))
            .unwrap();
        let th2 = shuttle::thread::Builder::new()
            .name("tread2".to_string())
            .spawn(gen_thread(a2))
            .unwrap();
        let th3 = shuttle::thread::Builder::new()
            .name("tread3".to_string())
            .spawn(gen_thread(a2))
            .unwrap();
        let starts = (
            th1.join().unwrap(),
            th2.join().unwrap(),
            th3.join().unwrap(),
        );
        // ensure we unmap the pages we've allocated
        reset_alloc(a2);
        // at least 1 thread should have started with a null ptr
        // and the other threads should have the same start pointer
        match starts {
            (0, th2, th3) => assert_eq!(th2, th3),
            (th1, 0, th3) => assert_eq!(th1, th3),
            (th1, th2, 0) => assert_eq!(th1, th2),
            (th1, th2, th3) => {
                panic!("expected one thread to start with a null pointer found\n\
                    th1: {th1}\n\
                    th2: {th2}\n\
                    th3: {th3}\n\
                ")
            }
        }
    }

    #[test]
    fn concurrent_allocs() {
        shuttle::check_random(concurrent_inner, *CONCURRENT_ITER);
    }

    #[test]
    fn concurrent_allocs_dfs() {
        shuttle::check_dfs(concurrent_inner, None);
    }

    #[test]
    fn concurrent_allocs_pct() {
        shuttle::check_pct(concurrent_inner, *CONCURRENT_ITER, 1000);
    }

    #[test]
    fn concurrent_allocs_nondeterminism() {
        shuttle::check_uncontrolled_nondeterminism(concurrent_inner, *CONCURRENT_ITER);
    }
}
