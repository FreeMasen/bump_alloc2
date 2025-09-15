use std::{collections::BTreeMap, u8};

#[derive(Debug, Default)]
pub struct Something {
    pub field1: u64,
    pub field2: u32,
    pub field3: u16,
    pub field4: u8,
    pub field5: bool,
}

impl Something {
    pub fn new_uniform(v: u8) -> Self {
        Self {
            field1: u64::from(v),
            field2: u32::from(v),
            field3: u16::from(v),
            field4: v,
            field5: v != 0,
        }
    }

    pub fn new_saturating_uniform(v: u16) -> Self {
        Self {
            field1: u64::from(v),
            field2: u32::from(v),
            field3: u16::from(v),
            field4: u8::try_from(v).unwrap_or(u8::MAX),
            field5: v != 0,
        }
    }

    pub fn new_random() -> Self {
        Self {
            field1: rand::random(),
            field2: rand::random(),
            field3: rand::random(),
            field4: rand::random(),
            field5: rand::random(),
        }
    }
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
            field3: u16::from(v),
            field4: u8::try_from(v).unwrap_or(u8::MAX),
            field5: v != 0,
        }
    }
}

#[track_caller]
pub fn vec100() {
    let v = (0..100)
        .map(|i| Something::new_uniform(i))
        .collect::<Vec<_>>();
    for (i, s) in v.into_iter().enumerate() {
        assert_eq!(i as u64, s.field1);
        assert_eq!(i as u32, s.field2);
        assert_eq!(i as u16, s.field3);
        assert_eq!(i as u8, s.field4);
        assert_eq!(i > 0, s.field5);
    }
}

#[track_caller]
pub fn btree_map_100() {
    let map = (0..100)
        .map(|i| (i, Something::new_uniform(i)))
        .collect::<BTreeMap<_, _>>();
    for (i, s) in map.into_iter() {
        assert_eq!(i as u64, s.field1);
        assert_eq!(i as u32, s.field2);
        assert_eq!(i as u16, s.field3);
        assert_eq!(i as u8, s.field4);
        assert_eq!(i > 0, s.field5);
    }
}

#[track_caller]
pub fn box_100() {
    let mut boxes: [Option<Box<Something>>; 100] = [const { None }; 100];
    for i in 0..100 {
        boxes[i] = Some(Box::new(Something::new_uniform(i as u8)));
    }

    for (i, s) in boxes.into_iter().enumerate() {
        let s = s.unwrap();
        assert_eq!(i as u64, s.field1);
        assert_eq!(i as u32, s.field2);
        assert_eq!(i as u16, s.field3);
        assert_eq!(i as u8, s.field4);
        assert_eq!(i > 0, s.field5);
    }
}

#[track_caller]
pub fn vec_u16_max() {
    let v = (0..u16::MAX)
        .map(|i| Something::from(i))
        .collect::<Vec<_>>();
    for (i, s) in v.into_iter().enumerate() {
        assert_eq!(i as u64, s.field1);
        assert_eq!(i as u32, s.field2);
        assert_eq!(i as u16, s.field3);
        assert_eq!(u8::try_from(i).unwrap_or(u8::MAX), s.field4);
        assert_eq!(i > 0, s.field5);
    }
}

#[track_caller]
pub fn btree_map_u16_max() {
    let map = (0..(u16::MAX))
        .map(|i| (i, Something::from(i)))
        .collect::<BTreeMap<_, _>>();
    for (i, s) in map.into_iter() {
        assert_eq!(i as u64, s.field1);
        assert_eq!(i as u32, s.field2);
        assert_eq!(i as u16, s.field3);
        assert_eq!(u8::try_from(i).unwrap_or(u8::MAX), s.field4);
        assert_eq!(i > 0, s.field5);
    }
}

#[track_caller]
pub fn box_u16_max() {
    let boxes: &mut [Option<Box<Something>>; u16::MAX as usize] =
        &mut [const { None }; u16::MAX as usize];
    for i in 0..u16::MAX {
        boxes[i as usize] = Some(Box::new(Something::from(i)));
    }

    for (i, s) in boxes.into_iter().enumerate() {
        let s = s.as_ref().unwrap();
        assert_eq!(i as u64, s.field1);
        assert_eq!(i as u32, s.field2);
        assert_eq!(i as u16, s.field3);
        assert_eq!(u8::try_from(i).unwrap_or(u8::MAX), s.field4);
        assert_eq!(i > 0, s.field5);
    }
}
