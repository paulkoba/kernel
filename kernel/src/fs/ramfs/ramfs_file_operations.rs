use crate::fs::file_operations::FileOperations;

pub static RAMFS_FILE_OPERATIONS: FileOperations = FileOperations {
    open: None,
    release: None,
    read: None,
    write: None,
};
