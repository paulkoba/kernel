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

#[no_mangle]
pub extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    unsafe {
        for i in 0..n {
            let byte1 = *s1.add(i);
            let byte2 = *s2.add(i);
            if byte1 != byte2 {
                return byte1 as i32 - byte2 as i32;
            }
        }
    }
    0
}
