use crate::fs::dentry::Dentry;
use crate::fs::file_operations::FileOperations;
use crate::fs::inode_operations::InodeOperations;
use crate::fs::super_block::SuperBlock;
use crate::types::{Gid, Mode, Uid};
use alloc::collections::LinkedList;

pub struct Inode {
    pub i_ino: u64,
    pub i_count: u32,
    pub i_mode: Mode,
    pub i_uid: Uid,
    pub i_gid: Gid,
    pub i_size: u64,           // File size in bytes
    pub i_sb: *mut SuperBlock, // Superblock this inode belongs to
    pub file_operations: Option<&'static FileOperations>,
    pub inode_operations: Option<&'static InodeOperations>,
    pub i_dentry: LinkedList<*mut Dentry>,
    // Filesystem-specific private data (opaque pointer)
    // For RAMFS, this will point to file data storage
    pub i_private: *mut u8,
}
