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
            // Allocate space on stack for strings and buffers
            "sub rsp, 256",
            // First, write a message to stdout
            "lea rsi, [rsp + 200]",
            "mov byte ptr [rsi], 'T'",
            "mov byte ptr [rsi + 1], 'e'",
            "mov byte ptr [rsi + 2], 's'",
            "mov byte ptr [rsi + 3], 't'",
            "mov byte ptr [rsi + 4], 'i'",
            "mov byte ptr [rsi + 5], 'n'",
            "mov byte ptr [rsi + 6], 'g'",
            "mov byte ptr [rsi + 7], ' '",
            "mov byte ptr [rsi + 8], 'f'",
            "mov byte ptr [rsi + 9], 'i'",
            "mov byte ptr [rsi + 10], 'l'",
            "mov byte ptr [rsi + 11], 'e'",
            "mov byte ptr [rsi + 12], ' '",
            "mov byte ptr [rsi + 13], 'o'",
            "mov byte ptr [rsi + 14], 'p'",
            "mov byte ptr [rsi + 15], 's'",
            "mov byte ptr [rsi + 16], '!'",
            "mov byte ptr [rsi + 17], 10", // newline
            "mov rax, 1",                  // sys_write
            "mov rdi, 1",                  // stdout
            "mov rdx, 18",                 // length
            "syscall",
            // Prepare path string "/dummy.txt" at rsp+100
            "lea rbx, [rsp + 100]",
            "mov byte ptr [rbx], '/'",
            "mov byte ptr [rbx + 1], 'd'",
            "mov byte ptr [rbx + 2], 'u'",
            "mov byte ptr [rbx + 3], 'm'",
            "mov byte ptr [rbx + 4], 'm'",
            "mov byte ptr [rbx + 5], 'y'",
            "mov byte ptr [rbx + 6], '.'",
            "mov byte ptr [rbx + 7], 't'",
            "mov byte ptr [rbx + 8], 'x'",
            "mov byte ptr [rbx + 9], 't'",
            "mov byte ptr [rbx + 10], 0", // null terminator
            // sys_open("/dummy.txt", O_RDONLY=0, 0)
            "mov rax, 2",   // sys_open
            "mov rdi, rbx", // pathname
            "mov rsi, 0",   // O_RDONLY
            "mov rdx, 0",   // mode
            "syscall",
            // Check if open succeeded (fd >= 3)
            "cmp rax, 3",
            "jl 2f", // jump to label 2 (open_failed)
            // Save fd in r12
            "mov r12, rax",
            // Read from file into buffer at rsp+150
            "mov rax, 0",           // sys_read
            "mov rdi, r12",         // fd
            "lea rsi, [rsp + 150]", // buffer
            "mov rdx, 64",          // count
            "syscall",
            // Save read count in r13
            "mov r13, rax",
            // Write what we read to stdout
            "mov rax, 1",           // sys_write
            "mov rdi, 1",           // stdout
            "lea rsi, [rsp + 150]", // buffer
            "mov rdx, r13",         // count
            "syscall",
            // Write a newline
            "lea rsi, [rsp + 200]",
            "mov byte ptr [rsi], 10",
            "mov rax, 1",
            "mov rdi, 1",
            "mov rdx, 1",
            "syscall",
            // Close the file
            "mov rax, 3",   // sys_close
            "mov rdi, r12", // fd
            "syscall",
            // Now test writing to the file
            "mov rax, 2",   // sys_open
            "mov rdi, rbx", // pathname
            "mov rsi, 1",   // O_WRONLY
            "mov rdx, 0",   // mode
            "syscall",
            "cmp rax, 3",
            "jl 2f",        // jump to label 2 (write_test_failed)
            "mov r12, rax", // save fd
            // Write test data
            "lea rsi, [rsp + 200]",
            "mov byte ptr [rsi], 'W'",
            "mov byte ptr [rsi + 1], 'r'",
            "mov byte ptr [rsi + 2], 'o'",
            "mov byte ptr [rsi + 3], 't'",
            "mov byte ptr [rsi + 4], 'e'",
            "mov byte ptr [rsi + 5], ' '",
            "mov byte ptr [rsi + 6], 'f'",
            "mov byte ptr [rsi + 7], 'r'",
            "mov byte ptr [rsi + 8], 'o'",
            "mov byte ptr [rsi + 9], 'm'",
            "mov byte ptr [rsi + 10], ' '",
            "mov byte ptr [rsi + 11], 'u'",
            "mov byte ptr [rsi + 12], 's'",
            "mov byte ptr [rsi + 13], 'e'",
            "mov byte ptr [rsi + 14], 'r'",
            "mov byte ptr [rsi + 15], 's'",
            "mov byte ptr [rsi + 16], 'p'",
            "mov byte ptr [rsi + 17], 'a'",
            "mov byte ptr [rsi + 18], 'c'",
            "mov byte ptr [rsi + 19], 'e'",
            "mov byte ptr [rsi + 20], '!'",
            "mov byte ptr [rsi + 21], 10",
            "mov rax, 1",   // sys_write
            "mov rdi, r12", // fd
            "mov rdx, 22",  // count
            "syscall",
            // Close file
            "mov rax, 3", // sys_close
            "mov rdi, r12",
            "syscall",
            "jmp 3f", // jump to label 3 (done)
            "2:",     // open_failed / write_test_failed
            "3:",     // done
            // Exit
            "mov rax, 60", // sys_exit
            "mov rdi, 0",  // exit code
            "syscall",
            "4:",
            "jmp 4b",
        );
    }
    loop {}
}
