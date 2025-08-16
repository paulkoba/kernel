use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;

pub struct HeapAllocator {
    #[allow(dead_code)]
    heap_start: usize,
    heap_end: usize,
    current: UnsafeCell<usize>,
}

// TODO: AAAAAAAAAAAAAAAAAAAAA
unsafe impl Sync for HeapAllocator {}

impl HeapAllocator {
    pub const fn new(heap_start: usize, heap_size: u64) -> Self {
        let heap_end = heap_start + heap_size as usize;
        HeapAllocator {
            heap_start,
            heap_end,
            current: UnsafeCell::new(heap_start),
        }
    }
}

unsafe impl GlobalAlloc for HeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();

        let aligned_current = self.current.get().align_offset(align);

        let new_ptr = *self.current.get() + aligned_current;

        if new_ptr + size > self.heap_end {
            null_mut()
        } else {
            *self.current.get() = new_ptr + size;
            new_ptr as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
