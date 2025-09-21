// not all fns here will be used so we disable that for this module.
#![allow(unused)]
use std::collections::{BTreeMap, LinkedList};

#[derive(Debug, Default, Clone, Copy)]
pub struct Something {
    pub field1: u64,
    pub field2: u32,
    pub field3: u16,
    pub field4: u8,
    pub field5: bool,
}

impl From<u8> for Something {
    fn from(v: u8) -> Self {
        Self {
            field1: u64::from(v),
            field2: u32::from(v),
            field3: u16::from(v),
            field4: v,
            field5: v != 0,
        }
    }
}

impl From<u16> for Something {
    fn from(v: u16) -> Self {
        Self {
            field1: u64::from(v),
            field2: u32::from(v),
            field3: v,
            field4: u8::try_from(v).unwrap_or(u8::MAX),
            field5: v != 0,
        }
    }
}

impl From<u32> for Something {
    fn from(v: u32) -> Self {
        Self {
            field1: u64::from(v),
            field2: v,
            field3: u16::try_from(v).unwrap_or(u16::MAX),
            field4: u8::try_from(v).unwrap_or(u8::MAX),
            field5: v != 0,
        }
    }
}

impl From<u64> for Something {
    fn from(v: u64) -> Self {
        Self {
            field1: v,
            field2: u32::try_from(v).unwrap_or(u32::MAX),
            field3: u16::try_from(v).unwrap_or(u16::MAX),
            field4: u8::try_from(v).unwrap_or(u8::MAX),
            field5: v != 0,
        }
    }
}

impl From<usize> for Something {
    fn from(value: usize) -> Self {
        if let Ok(e) = u8::try_from(value) {
            return Self::from(e);
        }
        if let Ok(s) = u16::try_from(value) {
            return Self::from(s);
        }
        if let Ok(t) = u32::try_from(value) {
            return Self::from(t);
        }
        Self::from(value as u64)
    }
}

#[track_caller]
pub fn check_iter(i: impl Iterator<Item = Something>) {
    for (i, s) in i.enumerate() {
        assert_eq!(i as u64, s.field1);
        assert_eq!(u32::try_from(i).unwrap_or(u32::MAX), s.field2);
        assert_eq!(u16::try_from(i).unwrap_or(u16::MAX), s.field3);
        assert_eq!(u8::try_from(i).unwrap_or(u8::MAX), s.field4);
        assert_eq!(i > 0, s.field5);
    }
}

#[track_caller]
pub fn vec100() {
    let v = (0u8..100).map(Something::from).collect::<Vec<_>>();
    check_iter(v.into_iter());
}

#[track_caller]
pub fn btree_map_100() {
    let map = (0u8..100)
        .map(|i| (i, Something::from(i)))
        .collect::<BTreeMap<_, _>>();
    check_iter(map.values().cloned());
}

#[track_caller]
pub fn box_100() {
    boxes::<100>();
}

#[track_caller]
pub fn vec_u16_max() {
    let v = (0..u16::MAX).map(Something::from).collect::<Vec<_>>();
    check_iter(v.into_iter());
}

#[track_caller]
pub fn btree_map_u16_max() {
    let map = (0..(u16::MAX))
        .map(|i| (i, Something::from(i)))
        .collect::<BTreeMap<_, _>>();
    check_iter(map.values().cloned());
}

#[track_caller]
pub fn box_u16_max() {
    boxes::<65535>();
}

#[track_caller]
pub fn boxes<const N: usize>() {
    let boxes: &mut [Option<Box<Something>>; N] = &mut [const { None }; N];
    for (i, b) in boxes.iter_mut().enumerate() {
        *b = Some(Box::new(Something::from(i)));
    }
    check_iter(boxes.iter_mut().filter_map(|b| b.take()).map(|b| *b));
}

#[track_caller]
pub fn linked_list<const N: usize>() {
    let list: LinkedList<_> = (0..N).map(Something::from).collect();
    check_iter(list.into_iter());
}
