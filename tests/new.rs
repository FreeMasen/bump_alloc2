mod shared;

use bump_alloc2::BumpAlloc;

#[global_allocator]
static A: BumpAlloc = BumpAlloc::new();

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
fn linked_list_100() {
    shared::linked_list::<100>();
}

#[cfg(not(miri))]
#[test]
fn vec_u16_max_works() {
    shared::vec_u16_max();
}

#[cfg(not(miri))]
#[test]
fn boxes_u16_max_works() {
    shared::box_u16_max();
}
