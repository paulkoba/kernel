use crate::fs::file::File;
use crate::fs::inode::Inode;

type OpenFn = unsafe extern "C" fn(inode: *mut Inode, file: *mut File) -> isize;
type ReleaseFn = unsafe extern "C" fn(inode: *mut Inode, file: *mut File) -> isize;
type ReadFn =
    unsafe extern "C" fn(file: *mut File, buf: *mut u8, count: usize, pos: *mut u64) -> isize;
type WriteFn =
    unsafe extern "C" fn(file: *mut File, buf: *const u8, count: usize, pos: *mut u64) -> isize;

pub struct FileOperations {
    pub open: Option<OpenFn>,
    pub release: Option<ReleaseFn>,
    pub read: Option<ReadFn>,
    pub write: Option<WriteFn>,
}
