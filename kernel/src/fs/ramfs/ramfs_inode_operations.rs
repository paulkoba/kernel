use crate::fs::inode_operations::InodeOperations;

pub static RAMFS_INODE_OPERATIONS: InodeOperations = InodeOperations {
    create: None,
    lookup: None,
    mkdir: None,
    rmdir: None,
    unlink: None,
    link: None,
    symlink: None,
    rename: None,
};
