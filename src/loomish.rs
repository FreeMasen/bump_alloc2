

pub use inner::*;

#[cfg(loom)]
pub mod inner {
    pub use loom::{
        alloc::Layout,
        sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
    };
}

#[cfg(not(loom))]
pub mod inner {
    pub use std::{
        alloc::Layout,
        sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
    };
}
