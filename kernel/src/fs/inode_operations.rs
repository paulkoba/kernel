use crate::fs::dentry::Dentry;
use crate::fs::inode::Inode;
use crate::types::{Gid, Mode, Uid};

type LookupFn = unsafe extern "C" fn(
    dir: *mut Inode,
    dentry: *mut Dentry,
    name: *const u8,
    namelen: usize,
) -> isize;
type CreateFn = unsafe extern "C" fn(
    dir: *mut Inode,
    dentry: *mut Dentry,
    mode: Mode,
    uid: Uid,
    gid: Gid,
) -> isize;

type MkdirFn = unsafe extern "C" fn(dir: *mut Inode, dentry: *mut Dentry, mode: Mode) -> isize;

type UnlinkFn = unsafe extern "C" fn(dir: *mut Inode, dentry: *mut Dentry) -> isize;

type LinkFn = unsafe extern "C" fn(
    old_dentry: *mut Dentry,
    new_dir: *mut Inode,
    new_dentry: *mut Dentry,
) -> isize;

type SymlinkFn =
    unsafe extern "C" fn(dir: *mut Inode, dentry: *mut Dentry, symname: *const u8) -> isize;

type RmdirFn = unsafe extern "C" fn(dir: *mut Inode, dentry: *mut Dentry) -> isize;

type RenameFn = unsafe extern "C" fn(
    old_dir: *mut Inode,
    old_dentry: *mut Dentry,
    new_dir: *mut Inode,
    new_dentry: *mut Dentry,
) -> isize;

pub struct InodeOperations {
    pub lookup: Option<LookupFn>,
    pub create: Option<CreateFn>,
    pub mkdir: Option<MkdirFn>,
    pub unlink: Option<UnlinkFn>,
    pub link: Option<LinkFn>,
    pub symlink: Option<SymlinkFn>,
    pub rmdir: Option<RmdirFn>,
    pub rename: Option<RenameFn>,
}
