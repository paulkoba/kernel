use crate::fs::inode::Inode;
use crate::fs::super_block::SuperBlock;
use alloc::collections::BTreeMap;
use alloc::string::String;

pub struct Dentry {
    pub d_name: String,
    pub d_inode: *mut Inode,
    pub d_sb: *mut SuperBlock,
    pub d_op: Option<&'static crate::fs::dentry_operations::DentryOperations>,
    pub d_parent: *mut Dentry, // Parent dentry (null for root)
    pub d_subdirs: BTreeMap<String, *mut Dentry>, // Child dentries
}
