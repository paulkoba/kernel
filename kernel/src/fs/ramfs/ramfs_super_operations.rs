use crate::fs::super_operations::SuperOperations;

pub static RAMFS_SUPER_OPERATIONS: SuperOperations = SuperOperations {
    read_inode: None,
    write_inode: None,
    put_inode: None,
    put_super: None,
    write_super: None,
    statfs: None,
    remount_fs: None,
};
