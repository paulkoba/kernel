use crate::fs::dentry::Dentry;
use crate::fs::file::File;
use crate::fs::inode::Inode;
use crate::fs::ramfs::ramfs;
use crate::fs::super_block::SuperBlock;
use crate::fs::super_operations::SuperOperations;
use crate::types::{FMode, Gid, Mode, Uid};
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, LinkedList};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

type MountFunc = fn(fs: &mut Filesystem, dev: u32, mount_point: &str) -> *mut Dentry;
type KillSbFunc = fn(sb: &mut SuperBlock) -> i32;

pub struct Filesystem {
    pub name: &'static str,
    pub mount: Option<MountFunc>,
    pub kill_sb: Option<KillSbFunc>,
    pub super_operations: Option<&'static SuperOperations>,
    pub fs_supers: LinkedList<SuperBlock>,
}

pub static mut FILESYSTEMS: LinkedList<Filesystem> = LinkedList::new();

pub fn register_filesystem(fs: Filesystem) {
    unsafe {
        FILESYSTEMS.push_back(fs);
    }
}

pub fn get_filesystem_by_name(name: &str) -> Option<&'static mut Filesystem> {
    unsafe {
        for fs in FILESYSTEMS.iter_mut() {
            if fs.name == name {
                return Some(fs);
            }
        }
    }
    None
}

pub fn mount_filesystem(fs_name: &str, dev: u32, mount_point: &str) -> *mut Dentry {
    if let Some(fs) = get_filesystem_by_name(fs_name) {
        if let Some(mount_func) = fs.mount {
            return mount_func(fs, dev, mount_point);
        }
    }
    core::ptr::null_mut()
}

pub fn get_full_path(dentry: *mut Dentry) -> String {
    let mut components = Vec::new();
    unsafe {
        let mut current = dentry;
        while !current.is_null() {
            let dentry_ref = &*current;
            components.push(dentry_ref.d_name.clone());
            current = dentry_ref.d_parent;
        }
    }

    if components.is_empty() {
        return String::from("/");
    }

    // Build path from components (they're in reverse order)
    let mut path = String::from("/");
    for (i, component) in components.iter().rev().enumerate() {
        if i > 0 {
            path.push('/');
        }
        path.push_str(component);
    }

    path
}

pub static mut ROOT_DENTRY: *mut Dentry = core::ptr::null_mut();

pub fn vfs_init() {
    ramfs::init_ramfs();

    unsafe {
        ROOT_DENTRY = mount_filesystem("ramfs", 1, "/");
    }
}

pub fn resolve_path(path: &str) -> *mut Dentry {
    unsafe {
        if ROOT_DENTRY.is_null() {
            return core::ptr::null_mut();
        }

        // Handle root path
        if path == "/" || path.is_empty() {
            return ROOT_DENTRY;
        }

        let mut current_dentry = ROOT_DENTRY;
        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        for component in components {
            let dentry_ref = &*current_dentry;

            // Check if inode exists and has lookup operation
            if !dentry_ref.d_inode.is_null() {
                let inode_ref = &*dentry_ref.d_inode;
                if let Some(inode_ops) = inode_ref.inode_operations {
                    // Try filesystem-specific lookup first
                    if let Some(lookup_fn) = inode_ops.lookup {
                        // For now, we'll use the VFS lookup through d_subdirs
                        // Filesystem-specific lookup can be enhanced later
                    }
                }
            }

            // VFS lookup through d_subdirs
            if let Some(child_dentry) = dentry_ref.d_subdirs.get(component) {
                current_dentry = *child_dentry;
            } else {
                return core::ptr::null_mut();
            }
        }

        current_dentry
    }
}

pub fn mkdir(parent: *mut Dentry, name: &str, mode: Mode, uid: Uid, gid: Gid) -> *mut Dentry {
    unsafe {
        if parent.is_null() {
            return core::ptr::null_mut();
        }

        let parent_ref = &mut *parent;

        // Check if directory already exists
        if parent_ref.d_subdirs.contains_key(name) {
            return core::ptr::null_mut();
        }

        // Check if parent has an inode
        if parent_ref.d_inode.is_null() {
            return core::ptr::null_mut();
        }

        let parent_inode = &*parent_ref.d_inode;

        // Check if parent inode has inode operations
        if parent_inode.inode_operations.is_none() {
            return core::ptr::null_mut();
        }

        let inode_op = parent_inode.inode_operations.unwrap();

        // Create new dentry
        let new_dentry = Box::new(Dentry {
            d_name: String::from(name),
            d_inode: core::ptr::null_mut(),
            d_sb: parent_ref.d_sb,
            d_op: parent_ref.d_op,
            d_parent: parent,
            d_subdirs: alloc::collections::BTreeMap::new(),
        });

        let new_dentry_ptr = Box::into_raw(new_dentry);

        // Call filesystem-specific mkdir operation
        if let Some(mkdir_fn) = inode_op.mkdir {
            let result = mkdir_fn(parent_ref.d_inode, new_dentry_ptr, mode);
            if result < 0 {
                // mkdir failed, clean up dentry
                let _ = Box::from_raw(new_dentry_ptr);
                return core::ptr::null_mut();
            }
        } else {
            // No mkdir operation, use generic create
            if let Some(create) = inode_op.create {
                let result = create(parent_ref.d_inode, new_dentry_ptr, mode, uid, gid);
                if result < 0 {
                    // create failed, clean up dentry
                    let _ = Box::from_raw(new_dentry_ptr);
                    return core::ptr::null_mut();
                }
            } else {
                // No operations available, clean up and return null
                let _ = Box::from_raw(new_dentry_ptr);
                return core::ptr::null_mut();
            }
        }

        // Add to parent's subdirs
        parent_ref
            .d_subdirs
            .insert(String::from(name), new_dentry_ptr);

        new_dentry_ptr
    }
}

pub fn create_file(parent: *mut Dentry, name: &str, mode: Mode, uid: Uid, gid: Gid) -> *mut Dentry {
    unsafe {
        if parent.is_null() {
            return core::ptr::null_mut();
        }

        let parent_ref = &mut *parent;

        // Check if file already exists
        if parent_ref.d_subdirs.contains_key(name) {
            return core::ptr::null_mut();
        }

        // Check if parent has an inode
        if parent_ref.d_inode.is_null() {
            return core::ptr::null_mut();
        }

        let parent_inode = &*parent_ref.d_inode;

        // Check if parent inode has inode operations
        if parent_inode.inode_operations.is_none() {
            return core::ptr::null_mut();
        }

        let inode_op = parent_inode.inode_operations.unwrap();

        // Create new dentry
        let new_dentry = Box::new(Dentry {
            d_name: String::from(name),
            d_inode: core::ptr::null_mut(),
            d_sb: parent_ref.d_sb,
            d_op: parent_ref.d_op,
            d_parent: parent,
            d_subdirs: alloc::collections::BTreeMap::new(),
        });

        let new_dentry_ptr = Box::into_raw(new_dentry);

        // Call filesystem-specific create operation
        if let Some(create_fn) = inode_op.create {
            let result = create_fn(parent_ref.d_inode, new_dentry_ptr, mode, uid, gid);
            if result < 0 {
                // create failed, clean up dentry
                let _ = Box::from_raw(new_dentry_ptr);
                return core::ptr::null_mut();
            }
        } else {
            // No create operation available, clean up and return null
            let _ = Box::from_raw(new_dentry_ptr);
            return core::ptr::null_mut();
        }

        // Add to parent's subdirs
        parent_ref
            .d_subdirs
            .insert(String::from(name), new_dentry_ptr);

        new_dentry_ptr
    }
}

pub fn allocate_empty_dentry(name: &str) -> *mut Dentry {
    let dentry = Box::new(Dentry {
        d_name: String::from(name),
        d_inode: core::ptr::null_mut(),
        d_sb: core::ptr::null_mut(),
        d_op: None,
        d_parent: core::ptr::null_mut(),
        d_subdirs: BTreeMap::new(),
    });
    Box::into_raw(dentry)
}

pub static mut INODES_LIST: BTreeMap<u64, *mut Inode> = BTreeMap::new();
pub static mut NEXT_INODE_NUMBER: u64 = 2; // Start at 2 since 1 is reserved for root
pub static MAX_INODES: u64 = 65536;
pub fn allocate_empty_inode(mode: Mode, uid: Uid, gid: Gid, sb: *mut SuperBlock) -> *mut Inode {
    let ino = unsafe {
        while INODES_LIST.contains_key(&NEXT_INODE_NUMBER) {
            NEXT_INODE_NUMBER += 1;
            if NEXT_INODE_NUMBER == MAX_INODES {
                NEXT_INODE_NUMBER = 1;
            }
        }
        NEXT_INODE_NUMBER
    };

    unsafe {
        let inode = Box::new(Inode {
            i_ino: ino,
            i_count: 1,
            i_mode: mode,
            i_uid: uid,
            i_gid: gid,
            i_size: 0,
            i_sb: sb,
            file_operations: None,
            inode_operations: None,
            i_dentry: LinkedList::new(),
            i_private: core::ptr::null_mut(),
        });
        let inode_ptr = Box::into_raw(inode);
        INODES_LIST.insert(ino, inode_ptr);
        inode_ptr
    }
}

// Helper functions for file operations
pub fn open_file(dentry: *mut Dentry, mode: FMode) -> Option<Box<File>> {
    unsafe {
        if dentry.is_null() {
            return None;
        }

        let dentry_ref = &*dentry;
        if dentry_ref.d_inode.is_null() {
            return None;
        }

        let inode = dentry_ref.d_inode;
        let inode_ref = &*inode;

        if inode_ref.file_operations.is_none() {
            return None;
        }

        let file_ops = inode_ref.file_operations.unwrap();
        let mut file = Box::new(File {
            f_inode: inode,
            f_mode: mode,
            f_pos: 0,
        });

        // Call open operation if available
        if let Some(open_fn) = file_ops.open {
            let result = open_fn(inode, file.as_mut() as *mut File);
            if result < 0 {
                return None;
            }
        }

        Some(file)
    }
}

pub fn read_file(file: &mut File, buf: &mut [u8]) -> isize {
    unsafe {
        if file.f_inode.is_null() {
            return -1;
        }

        let inode_ref = &*file.f_inode;
        if inode_ref.file_operations.is_none() {
            return -1;
        }

        let file_ops = inode_ref.file_operations.unwrap();
        if let Some(read_fn) = file_ops.read {
            read_fn(
                file as *mut File,
                buf.as_mut_ptr(),
                buf.len(),
                &mut file.f_pos,
            )
        } else {
            -1
        }
    }
}

pub fn write_file(file: &mut File, buf: &[u8]) -> isize {
    unsafe {
        if file.f_inode.is_null() {
            return -1;
        }

        let inode_ref = &*file.f_inode;
        if inode_ref.file_operations.is_none() {
            return -1;
        }

        let file_ops = inode_ref.file_operations.unwrap();
        if let Some(write_fn) = file_ops.write {
            write_fn(file as *mut File, buf.as_ptr(), buf.len(), &mut file.f_pos)
        } else {
            -1
        }
    }
}

pub fn close_file(mut file: Box<File>) {
    unsafe {
        if file.f_inode.is_null() {
            return;
        }

        let inode_ref = &*file.f_inode;
        if let Some(file_ops) = inode_ref.file_operations {
            if let Some(release_fn) = file_ops.release {
                let file_ptr = file.as_mut() as *mut File;
                release_fn(file.f_inode, file_ptr);
            }
        }
        // File is dropped here
    }
}
