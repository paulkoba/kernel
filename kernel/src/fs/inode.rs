use crate::fs::file_operations::FileOperations;
use crate::fs::inode_operations::InodeOperations;
use crate::types::{Gid, Mode, Uid};

pub struct Inode {
    pub i_ino: u64,
    pub i_count: u32,
    pub i_mode: Mode,
    pub i_uid: Uid,
    pub i_gid: Gid,
    pub file_operations: Option<&'static FileOperations>,
    pub inode_operations: Option<&'static InodeOperations>,
}
