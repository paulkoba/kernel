use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

static mut RAMFS_DATA: BTreeMap<u64, Vec<u8>> = BTreeMap::new();

#[inline(always)]
unsafe fn data_map() -> &'static mut BTreeMap<u64, Vec<u8>> {
    &mut RAMFS_DATA
}

pub unsafe fn ramfs_get_data(ino: u64) -> Option<&'static mut Vec<u8>> {
    data_map().get_mut(&ino).map(|v| core::mem::transmute(v))
}

pub unsafe fn ramfs_set_data(ino: u64, data: Vec<u8>) {
    data_map().insert(ino, data);
}

pub unsafe fn ramfs_allocate_data(ino: u64) -> &'static mut Vec<u8> {
    let map = data_map();
    let entry = map.entry(ino).or_insert_with(Vec::new);
    core::mem::transmute(entry)
}

pub unsafe fn ramfs_remove_data(ino: u64) {
    data_map().remove(&ino);
}

pub unsafe fn ramfs_try_remove_data(ino: u64) -> bool {
    data_map().remove(&ino).is_some()
}

pub unsafe fn ramfs_resize_data(ino: u64, new_size: usize) {
    let map = data_map();
    if let Some(data) = map.get_mut(&ino) {
        data.resize(new_size, 0);
    } else {
        map.insert(ino, vec![0; new_size]);
    }
}
