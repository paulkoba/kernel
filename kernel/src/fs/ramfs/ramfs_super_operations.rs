use crate::fs::inode::Inode;
use crate::fs::ramfs::ramfs_data::ramfs_remove_data;
use crate::fs::super_operations::SuperOperations;

pub static RAMFS_SUPER_OPERATIONS: SuperOperations = SuperOperations {
    read_inode: None,
    write_inode: None,
    put_inode: None,
    put_super: None,
    write_super: None,
    statfs: None,
    remount_fs: None,
    drop_inode: Some(ramfs_drop_inode),
};
unsafe extern "C" fn ramfs_drop_inode(inode: *mut Inode) {
    if inode.is_null() {
        return;
    }

    unsafe {
        ramfs_remove_data((*inode).i_ino);
    }
}
