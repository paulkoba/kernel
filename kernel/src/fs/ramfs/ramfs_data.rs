// Centralized file data storage for RAMFS
// Maps inode numbers to file data

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// Global storage for file data
static mut RAMFS_DATA: Option<BTreeMap<u64, Vec<u8>>> = None;
static INIT_FLAG: AtomicU64 = AtomicU64::new(0);

unsafe fn ensure_initialized() {
    if INIT_FLAG
        .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        RAMFS_DATA = Some(BTreeMap::new());
    }
}

pub unsafe fn ramfs_get_data(ino: u64) -> Option<&'static mut Vec<u8>> {
    ensure_initialized();
    if let Some(ref mut data_map) = RAMFS_DATA {
        // SAFETY: The data_map is static, so we can safely extend the lifetime
        // This is acceptable in kernel code where we control the memory layout
        data_map.get_mut(&ino).map(|v| core::mem::transmute(v))
    } else {
        None
    }
}

pub unsafe fn ramfs_set_data(ino: u64, data: Vec<u8>) {
    ensure_initialized();
    if let Some(ref mut data_map) = RAMFS_DATA {
        data_map.insert(ino, data);
    }
}

pub unsafe fn ramfs_allocate_data(ino: u64) -> &'static mut Vec<u8> {
    ensure_initialized();
    if let Some(ref mut data_map) = RAMFS_DATA {
        if !data_map.contains_key(&ino) {
            data_map.insert(ino, Vec::new());
        }
        // SAFETY: The data_map is static, so we can safely extend the lifetime
        // This is acceptable in kernel code where we control the memory layout
        core::mem::transmute(data_map.get_mut(&ino).unwrap())
    } else {
        panic!("RAMFS data storage not initialized");
    }
}

pub unsafe fn ramfs_remove_data(ino: u64) {
    ensure_initialized();
    if let Some(ref mut data_map) = RAMFS_DATA {
        data_map.remove(&ino);
    }
}

// Check if data exists for an inode and remove it if it does
pub unsafe fn ramfs_try_remove_data(ino: u64) -> bool {
    ensure_initialized();
    if let Some(ref mut data_map) = RAMFS_DATA {
        data_map.remove(&ino).is_some()
    } else {
        false
    }
}

pub unsafe fn ramfs_resize_data(ino: u64, new_size: usize) {
    ensure_initialized();
    if let Some(ref mut data_map) = RAMFS_DATA {
        if let Some(data) = data_map.get_mut(&ino) {
            data.resize(new_size, 0);
        } else {
            data_map.insert(ino, vec![0; new_size]);
        }
    }
}
