use crate::fs::inode::Inode;
use crate::fs::statfs::StatFs;
use crate::fs::super_block::SuperBlock;

type ReadInodeFn = unsafe extern "C" fn(inode: *mut Inode);
type WriteInodeFn = unsafe extern "C" fn(inode: *mut Inode);
type PutInodeFn = unsafe extern "C" fn(inode: *mut Inode);
type PutSuperFn = unsafe extern "C" fn(super_block: *mut SuperBlock);
type WriteSuperFn = unsafe extern "C" fn(super_block: *mut SuperBlock);
type StatfsFn = unsafe extern "C" fn(super_block: *mut SuperBlock, buf: *mut StatFs, bufsize: u32);
type RemountFsFn =
    unsafe extern "C" fn(super_block: *mut SuperBlock, flags: *mut u32, data: *mut u8) -> u32;

pub struct SuperOperations {
    pub read_inode: Option<ReadInodeFn>,
    pub write_inode: Option<WriteInodeFn>,
    pub put_inode: Option<PutInodeFn>,
    pub put_super: Option<PutSuperFn>,
    pub write_super: Option<WriteSuperFn>,
    pub statfs: Option<StatfsFn>,
    pub remount_fs: Option<RemountFsFn>,
}
