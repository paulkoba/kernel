use crate::fs::dentry::Dentry;
use crate::fs::super_operations::SuperOperations;
use crate::fs::vfs::Filesystem;
use crate::types::Dev;

pub struct SuperBlock {
    pub s_dev: Dev,
    pub s_root: *mut Dentry,
    pub s_op: Option<&'static SuperOperations>,
    pub s_fs: Option<&'static Filesystem>,
}
