#![cfg_attr(feature = "nightly", feature(allocator_api))]

#[cfg(not(feature = "nightly"))]
use allocator_api2::{boxed::Box, vec::Vec};
use bump_alloc::BumpAlloc;

use crate::shared::{Something, check_iter};

mod shared;

#[test]
fn vec100() {
    let alloc = BumpAlloc::default();
    let mut v = Vec::new_in(alloc);
    v.extend((0..100u8).map(Something::from));
    check_iter(v.into_iter());
}

#[test]
fn vec_u16_max() {
    let alloc = BumpAlloc::default();
    let mut v = Vec::new_in(alloc);
    v.extend((0..65535u16).map(Something::from));
    check_iter(v.into_iter());
}

#[test]
fn boxes100() {
    let alloc = BumpAlloc::default();
    boxes::<100>(&alloc);
}

#[test]
fn boxes_u16_max() {
    let alloc = BumpAlloc::default();
    boxes::<65535>(&alloc);
}

#[track_caller]
pub fn boxes<const N: usize>(alloc: &BumpAlloc) {
    let boxes: &mut [Option<Box<Something, _>>; N] = &mut [const { None }; N];
    for (i, b) in boxes.iter_mut().enumerate() {
        *b = Some(Box::new_in(Something::from(i), alloc));
    }
    shared::check_iter(boxes.iter_mut().filter_map(|b| b.take()).map(|b| *b));
}

#[cfg(feature = "nightly")]
#[test]
fn linked_list100() {
    let alloc = BumpAlloc::new();
    let mut linked_list = std::collections::LinkedList::new_in(alloc);
    for i in 0..100u8 {
        linked_list.push_back(Something::from(i))
    }
}

#[cfg(feature = "nightly")]
#[test]
fn linked_list_u16_max() {
    let alloc = BumpAlloc::new();
    let mut linked_list = std::collections::LinkedList::new_in(alloc);
    for i in 0..=u16::MAX {
        linked_list.push_back(Something::from(i))
    }
}
