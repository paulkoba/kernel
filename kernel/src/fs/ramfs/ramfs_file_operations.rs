use crate::fs::file::File;
use crate::fs::file_operations::FileOperations;
use crate::fs::inode::Inode;
use crate::fs::ramfs::ramfs_data;

unsafe extern "C" fn ramfs_read(
    file: *mut File,
    buf: *mut u8,
    count: usize,
    pos: *mut u64,
) -> isize {
    if file.is_null() || buf.is_null() || pos.is_null() {
        return -1;
    }

    let file_ref = &*file;
    let inode = file_ref.f_inode;
    if inode.is_null() {
        return -1;
    }

    let inode_ref = &*inode;
    let current_pos = *pos;
    let file_size = inode_ref.i_size as usize;

    // Check if we're at or past the end of the file
    if current_pos as usize >= file_size {
        return 0; // EOF
    }

    // Get the file data
    let data = ramfs_data::ramfs_get_data(inode_ref.i_ino);
    let data_vec = match data {
        Some(vec) => vec,
        None => return 0, // No data available
    };
    let available = file_size - (current_pos as usize);
    let to_read = if count > available { available } else { count };

    if to_read == 0 {
        return 0;
    }

    // Copy data to user buffer
    let start = current_pos as usize;
    let end = start + to_read;
    let src = &data_vec[start..end];
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), buf, to_read);
    }

    // Update position
    *pos = (current_pos as usize + to_read) as u64;

    to_read as isize
}

unsafe extern "C" fn ramfs_write(
    file: *mut File,
    buf: *const u8,
    count: usize,
    pos: *mut u64,
) -> isize {
    if file.is_null() || buf.is_null() || pos.is_null() {
        return -1;
    }

    let file_ref = &*file;
    let inode = file_ref.f_inode;
    if inode.is_null() {
        return -1;
    }

    let inode_ref = &mut *inode;
    let current_pos = *pos;

    // Get or allocate file data
    let data = ramfs_data::ramfs_allocate_data(inode_ref.i_ino);

    // Ensure the data buffer is large enough
    let required_size = (current_pos as usize) + count;
    if data.len() < required_size {
        data.resize(required_size, 0);
    }

    // Copy data from user buffer
    let start = current_pos as usize;
    unsafe {
        core::ptr::copy_nonoverlapping(buf, data.as_mut_ptr().add(start), count);
    }

    // Update file size if we wrote past the end
    let new_size = (current_pos as usize) + count;
    if new_size > inode_ref.i_size as usize {
        inode_ref.i_size = new_size as u64;
    }

    // Update position
    *pos = new_size as u64;

    count as isize
}

unsafe extern "C" fn ramfs_open(inode: *mut Inode, file: *mut File) -> isize {
    if inode.is_null() || file.is_null() {
        return -1;
    }

    // For RAMFS, we just need to ensure data storage exists
    let inode_ref = &*inode;
    ramfs_data::ramfs_allocate_data(inode_ref.i_ino);

    0
}

unsafe extern "C" fn ramfs_release(inode: *mut Inode, file: *mut File) -> isize {
    // For RAMFS, we don't need to do anything special on release
    // The data stays in memory
    0
}

pub static RAMFS_FILE_OPERATIONS: FileOperations = FileOperations {
    open: Some(ramfs_open),
    release: Some(ramfs_release),
    read: Some(ramfs_read),
    write: Some(ramfs_write),
};
