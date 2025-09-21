mod shared;

use bump_alloc2::BumpAlloc;

#[global_allocator]
static A: BumpAlloc = BumpAlloc::with_size(1024 * 1024 * 4);

#[test]
fn alloc_works() {
    use std::alloc::{Layout, alloc, dealloc};
    let layout = Layout::new::<u16>();
    let ptr = unsafe { alloc(layout) as *mut u16 };

    unsafe { *ptr = 42 };
    assert_eq!(unsafe { *ptr }, 42);

    unsafe { dealloc(ptr as *mut u8, layout) };
}

#[test]
fn vec100_works() {
    shared::vec100();
}

#[test]
fn btree_map100_works() {
    shared::btree_map_100();
}

#[test]
fn box100_works() {
    shared::box_100();
}

#[test]
fn vec_u16_max_works() {
    shared::vec_u16_max();
}

#[test]
fn boxes_u16_max_works() {
    shared::box_u16_max();
}

#[test]
fn linked_list_100() {
    shared::linked_list::<100>();
}
