use crate::fs::dentry::Dentry;
use crate::fs::inode::Inode;
use crate::fs::inode_operations::InodeOperations;
use crate::fs::ramfs::ramfs_file_operations;
use crate::fs::vfs;
use crate::types::{Gid, Mode, Uid};

unsafe extern "C" fn ramfs_mkdir(dir: *mut Inode, dentry: *mut Dentry, mode: Mode) -> isize {
    if dir.is_null() || dentry.is_null() {
        return -1;
    }

    let dir_ref = &*dir;
    let dentry_ref = &mut *dentry;

    // Ensure directory mode
    let dir_mode = Mode::from(mode.0 | 0o40000); // Set directory bit

    let new_inode = vfs::allocate_empty_inode(dir_mode, dir_ref.i_uid, dir_ref.i_gid, dir_ref.i_sb);
    if new_inode.is_null() {
        return -1;
    }

    unsafe {
        let new_inode_ref = &mut *new_inode;
        new_inode_ref.inode_operations = dir_ref.inode_operations;
        new_inode_ref.file_operations = Some(&ramfs_file_operations::RAMFS_FILE_OPERATIONS);
        new_inode_ref.i_size = 0;
        new_inode_ref.i_dentry.push_back(dentry);
    }

    dentry_ref.d_inode = new_inode;

    0
}

unsafe extern "C" fn ramfs_create(
    dir: *mut Inode,
    dentry: *mut Dentry,
    mode: Mode,
    uid: Uid,
    gid: Gid,
) -> isize {
    if dir.is_null() || dentry.is_null() {
        return -1;
    }

    let dir_ref = &*dir;
    let dentry_ref = &mut *dentry;

    // Allocate new inode
    let new_inode = vfs::allocate_empty_inode(mode, uid, gid, dir_ref.i_sb);
    if new_inode.is_null() {
        return -1;
    }

    unsafe {
        let new_inode_ref = &mut *new_inode;
        // Set inode operations for file (use parent's operations)
        new_inode_ref.inode_operations = dir_ref.inode_operations;
        new_inode_ref.file_operations = Some(&ramfs_file_operations::RAMFS_FILE_OPERATIONS);
        new_inode_ref.i_size = 0; // New file starts with size 0
                                  // Link dentry to inode
        new_inode_ref.i_dentry.push_back(dentry);
    }

    // Link dentry to inode
    dentry_ref.d_inode = new_inode;

    0
}

unsafe extern "C" fn ramfs_lookup(
    dir: *mut Inode,
    dentry: *mut Dentry,
    name: *const u8,
    namelen: usize,
) -> isize {
    if dir.is_null() || dentry.is_null() || name.is_null() {
        return -1;
    }

    // For ramfs, lookup is handled by the VFS layer through d_subdirs
    // This function can be used for filesystem-specific lookup logic
    // For now, we return success and let VFS handle it
    0
}

pub static RAMFS_INODE_OPERATIONS: InodeOperations = InodeOperations {
    create: Some(ramfs_create),
    lookup: Some(ramfs_lookup),
    mkdir: Some(ramfs_mkdir),
    rmdir: None,
    unlink: None,
    link: None,
    symlink: None,
    rename: None,
};
