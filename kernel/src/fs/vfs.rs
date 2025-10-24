use crate::fs::file_operations::FileOperations;
use crate::fs::inode_operations::InodeOperations;
use crate::fs::super_operations::SuperOperations;

pub struct Filesystem {
    pub name: &'static str,
    pub super_operations: Option<&'static SuperOperations>,
    pub inode_operations: Option<&'static InodeOperations>,
    pub file_operations: Option<&'static FileOperations>,
}
