use crate::fs::dentry::Dentry;
use crate::fs::ramfs::ramfs_file_operations;
use crate::fs::ramfs::ramfs_inode_operations;
use crate::fs::ramfs::ramfs_super_operations;
use crate::fs::super_block::SuperBlock;
use crate::fs::vfs;
use crate::fs::vfs::Filesystem;
use crate::klog;
use crate::types::{Dev, Gid, Mode, Uid};
use alloc::boxed::Box;
use alloc::string::String;

fn ramfs_mount(fs: &mut Filesystem, dev: u32, mount_point: &str) -> *mut Dentry {
    klog!(Debug, "Mounting ramfs with dev={}", dev);
    let fs_static: &'static Filesystem = unsafe { core::mem::transmute(fs) };

    let sb = Box::new(SuperBlock {
        s_dev: Dev::from(dev),
        s_root: core::ptr::null_mut(),
        s_op: Some(&ramfs_super_operations::RAMFS_SUPER_OPERATIONS),
        s_fs: Some(fs_static),
    });

    let sb_ptr = Box::into_raw(sb);

    let root_inode = Box::new(crate::fs::inode::Inode {
        i_ino: 1,
        i_count: 1,
        i_mode: Mode::from(0o40777),
        i_uid: Uid::from(0),
        i_gid: Gid::from(0),
        i_size: 0,
        i_sb: sb_ptr,
        file_operations: Some(&ramfs_file_operations::RAMFS_FILE_OPERATIONS),
        inode_operations: Some(&ramfs_inode_operations::RAMFS_INODE_OPERATIONS),
        i_dentry: alloc::collections::LinkedList::new(),
        i_private: core::ptr::null_mut(),
    });
    let root_inode_ptr = Box::into_raw(root_inode);

    unsafe {
        crate::fs::vfs::INODES_LIST.insert(1, root_inode_ptr);
    }

    let root_dentry = Box::new(Dentry {
        d_name: String::from(mount_point),
        d_inode: root_inode_ptr,
        d_sb: sb_ptr,
        d_op: None,
        d_parent: core::ptr::null_mut(),
        d_subdirs: alloc::collections::BTreeMap::new(),
    });
    let root_dentry_ptr = Box::into_raw(root_dentry);

    unsafe {
        (*sb_ptr).s_root = root_dentry_ptr;
        (*root_inode_ptr).i_dentry.push_back(root_dentry_ptr);
    }
    root_dentry_ptr
}

fn ramfs_kill_sb(sb: &mut SuperBlock) -> i32 {
    klog!(Debug, "Killing RAMFS superblock");

    sb.s_root = core::ptr::null_mut();

    0
}

pub fn init_ramfs() {
    let ramfs = Filesystem {
        name: "ramfs",
        mount: Some(ramfs_mount),
        kill_sb: Some(ramfs_kill_sb),
        fs_supers: alloc::collections::LinkedList::new(),
    };
    vfs::register_filesystem(ramfs);
}
