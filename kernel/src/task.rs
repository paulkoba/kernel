use crate::memory::create_user_page_table_with_mapper;
use x86_64::structures::paging::{FrameAllocator, OffsetPageTable, Size4KiB};
use x86_64::VirtAddr;

#[repr(C)]
#[derive(Debug)]
pub struct TrapFrame {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

pub struct Task {
    pub pid: u64,
    pub trap_frame: *mut TrapFrame,
    pub page_table: OffsetPageTable<'static>,
}

impl Task {
    pub fn new(
        pid: u64,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
        physical_memory_offset: VirtAddr,
    ) -> Self {
        Task {
            pid,
            trap_frame: core::ptr::null_mut(),
            page_table: create_user_page_table_with_mapper(frame_allocator, physical_memory_offset)
                .unwrap(),
        }
    }
}

static mut CURRENT_TASK: Option<Task> = None;

// need to use those getters since those will at some point become per-core
#[allow(static_mut_refs)]
pub fn get_current_task() -> &'static mut Task {
    unsafe { CURRENT_TASK.as_mut().unwrap() }
}

pub fn set_current_task(task: Task) {
    unsafe {
        CURRENT_TASK = Some(task);
    }
}
