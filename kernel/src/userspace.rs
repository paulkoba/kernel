use crate::gdt::SELECTORS;
use crate::logging::LogLevel;
use crate::memory;
use crate::memory::{USERSPACE_CODE_START, USERSPACE_STACK_START};
use crate::task::Task;
use crate::{klog, logging, syscall};
use core::arch::asm;
use core::fmt::Write;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::VirtAddr;

pub fn jump_userspace(frame_allocator: &mut impl FrameAllocator<Size4KiB>, task: Task) -> ! {
    let mut mapper = task.page_table;
    let user_stack_frame = frame_allocator
        .allocate_frame()
        .expect("no more frames available");
    let user_code_frame = frame_allocator
        .allocate_frame()
        .expect("no more frames available");

    let user_stack_flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::NO_EXECUTE;
    let user_stack_start = VirtAddr::new(memory::USERSPACE_STACK_START);
    let user_stack_page = Page::containing_address(user_stack_start);
    unsafe {
        mapper
            .map_to(
                user_stack_page,
                user_stack_frame,
                user_stack_flags,
                frame_allocator,
            )
            .expect("map_to failed")
            .flush();
    }

    let user_code_flags =
        PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE;
    let user_code_start = VirtAddr::new(USERSPACE_CODE_START);
    let user_code_page = Page::containing_address(user_code_start);
    unsafe {
        mapper
            .map_to(
                user_code_page,
                user_code_frame,
                user_code_flags,
                frame_allocator,
            )
            .expect("map_to failed")
            .flush();
    }

    let fn_size = 0x800;
    let userspace_fn = user_code_start.as_u64() as *mut u8;
    klog!(Debug, "1");
    unsafe {
        core::ptr::copy_nonoverlapping(test_userspace_routine as *const _, userspace_fn, fn_size);
    }
    klog!(Debug, "2");

    let user_stack_pointer = user_stack_page.start_address().as_u64() + 4096 - 2048;

    unsafe {
        asm!(
        "cli",
        "push {user_ds}",
        "push {user_sp}",
        "pushfq",
        "pop rax",
        "or rax, 0x200",
        "push rax",
        "push {user_cs}",
        "push {user_rip}",
        "iretq",

        user_ds = in(reg) SELECTORS.user_data_selector.0 as u64,
        user_cs = in(reg) SELECTORS.user_code_selector.0 as u64,
        user_sp = in(reg) user_stack_pointer,
        user_rip = in(reg) user_code_start.as_u64(),
        );
    }

    unreachable!("iretq failed");
}

#[no_mangle]
pub extern "C" fn test_userspace_routine() {
    static MSG_2: &[u8] = b"Userspace code running!\n";
    static MSG: &[u8] = b"Hello, World! Test........\n";

    unsafe {
        asm!(
        "mov rax, 1",
        "mov rdi, 1",
        "mov rsi, {msg_ptr}",
        "mov rdx, {msg_len}",
        "syscall",
        "mov rax, 1",
        "mov rsi, {msg2_ptr}",
        "mov rdx, {msg2_len}",
        "syscall",
        "mov rax, 39",
        "syscall",
        "mov rdi, rax",
        "mov rax, 60",
        "syscall",
        "2:",
        "jmp 2b",
        msg_ptr = in(reg) MSG.as_ptr(),
        msg_len = in(reg) MSG.len(),
        msg2_ptr = in(reg) MSG_2.as_ptr(),
        msg2_len = in(reg) MSG_2.len(),
        options(nomem, nostack)
        );
    }
    loop {}
}
