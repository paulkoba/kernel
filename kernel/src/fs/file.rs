use crate::fs::inode::Inode;
use crate::types::FMode;

pub struct File {
    pub f_inode: *mut Inode,
    pub f_mode: FMode,
    pub f_pos: u64,
}
