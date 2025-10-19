use crate::memory::create_user_page_table_with_mapper;
use alloc::collections::BTreeMap;
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

#[allow(dead_code)]
pub struct Task {
    pub pid: u64,
    pub ppid: u64,
    pub trap_frame: *mut TrapFrame,
    pub page_table: OffsetPageTable<'static>,
}

impl Task {
    pub fn new(
        pid: u64,
        ppid: u64,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
        physical_memory_offset: VirtAddr,
    ) -> Self {
        Task {
            pid,
            ppid,
            trap_frame: core::ptr::null_mut(),
            page_table: create_user_page_table_with_mapper(frame_allocator, physical_memory_offset)
                .unwrap(),
        }
    }
}

// task
static mut TASKS: BTreeMap<u64, Task> = BTreeMap::new();

static mut CURRENT_TASK: u64 = 0;
static mut NEXT_PID: u64 = 1;
static mut PID_MAX: u64 = 0x0400_0000;

#[allow(static_mut_refs)]

pub fn create_task(
    ppid: u64,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    physical_memory_offset: VirtAddr,
) -> u64 {
    unsafe {
        while TASKS.contains_key(&NEXT_PID) {
            NEXT_PID += 1;
            if NEXT_PID >= PID_MAX {
                NEXT_PID = 1;
            }
        }

        TASKS.insert(
            NEXT_PID,
            Task::new(NEXT_PID, ppid, frame_allocator, physical_memory_offset),
        );
        NEXT_PID
    }
}

// need to use those getters since those will at some point become per-core

pub fn getpid() -> u64 {
    unsafe { CURRENT_TASK }
}

pub fn getppid() -> u64 {
    let pid = getpid();
    #[allow(static_mut_refs)]
    unsafe {
        TASKS.get(&pid).map(|task| task.ppid).unwrap_or(0)
    }
}

pub fn get_current_task() -> Option<&'static mut Task> {
    let pid = getpid();
    #[allow(static_mut_refs)]
    unsafe {
        TASKS.get_mut(&pid)
    }
}

pub fn set_current_pid(pid: u64) {
    unsafe {
        CURRENT_TASK = pid;
    }
}
