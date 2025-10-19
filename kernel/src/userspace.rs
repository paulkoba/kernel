use crate::gdt::SELECTORS;
use crate::memory;
use crate::memory::USERSPACE_CODE_START;
use crate::task::Task;
use core::arch::asm;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::VirtAddr;

pub fn jump_userspace(frame_allocator: &mut impl FrameAllocator<Size4KiB>, task: &mut Task) -> () {
    let mapper = &mut task.page_table;
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
    unsafe {
        core::ptr::copy_nonoverlapping(test_userspace_routine as *const _, userspace_fn, fn_size);
    }

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
}

#[no_mangle]
pub extern "C" fn test_userspace_routine() {
    unsafe {
        asm!(
            "sub rsp, 13",
            "mov byte ptr [rsp], 'H'",
            "mov byte ptr [rsp + 1], 'e'",
            "mov byte ptr [rsp + 2], 'l'",
            "mov byte ptr [rsp + 3], 'l'",
            "mov byte ptr [rsp + 4], 'o'",
            "mov byte ptr [rsp + 5], ','",
            "mov byte ptr [rsp + 6], ' '",
            "mov byte ptr [rsp + 7], 'w'",
            "mov byte ptr [rsp + 8], 'o'",
            "mov byte ptr [rsp + 9], 'r'",
            "mov byte ptr [rsp + 10], 'l'",
            "mov byte ptr [rsp + 11], 'd'",
            "mov byte ptr [rsp + 12], '!'",
            "mov rax, 1",
            "mov rdi, 1",
            "mov rsi, rsp",
            "mov rdx, 13",
            "syscall",     // sys_write
            "mov rax, 39", // sys_getpid
            "syscall",
            "mov rdi, rax",
            "mov rax, 60",
            "syscall", // sys_exit
            "2:",
            "jmp 2b",
        );
    }
    loop {}
}
