#[no_mangle]
pub extern "C" fn memset(dest: *mut u8, val: u8, count: usize) -> *mut u8 {
    unsafe {
        let mut i = 0;
        while i < count {
            *dest.add(i) = val;
            i += 1;
        }
    }
    dest
}

#[no_mangle]
pub extern "C" fn memcpy(dest: *mut u8, src: *const u8, count: usize) -> *mut u8 {
    unsafe {
        let mut i = 0;
        while i < count {
            *dest.add(i) = *src.add(i);
            i += 1;
        }
    }
    dest
}
