use crate::gdt::SELECTORS;
use crate::instructions::{rdmsr, wrmsr, EFER, FMASK, KERNEL_GS_BASE, LSTAR, STAR};
use crate::{klog, logging, LogLevel};
use core::arch::asm;
use core::fmt::Write;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::VirtAddr;
unsafe fn configure_syscalls() {
    let syscall_handler_addr = syscall_handler as *const () as u64;
    let efer = rdmsr(EFER);

    let syscall_cs_ss_base = (SELECTORS.kernel_code_selector.0 & 0xFFFC) as u32;
    let sysret_cs_ss_base = ((SELECTORS.user_code_selector.0 & 0xFFFC) as u32);

    let star_high = (syscall_cs_ss_base) | (sysret_cs_ss_base << 16);

    wrmsr(STAR, (star_high as u64) << 32);
    wrmsr(LSTAR, syscall_handler_addr);
    wrmsr(FMASK, 0x0300);
    wrmsr(EFER, efer | 0x1);
    wrmsr(KERNEL_GS_BASE, SELECTORS.tss_selector.0 as u64);
}

pub fn jump_userspace(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> ! {
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
    let user_stack_start = VirtAddr::new(0x7fff_ffff_0000);
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
    let user_code_start = VirtAddr::new(0x4000_0000);
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

        configure_syscalls();
    }

    let user_stack_pointer = user_stack_page.start_address().as_u64() + 4096 - 2048;

    unsafe {
        asm!(
        "cli",
        "mov rax, {user_ds}",
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",

        "push {user_ds}",
        "push {user_sp}",
        "pushfq",
        "pop rax",
        //"or rax, 0x200",
        "push rax",
        "push {user_cs}",
        "push {user_rip}",
        "iretq",

        user_ds = in(reg) SELECTORS.user_data_selector.0,
        user_cs = in(reg) SELECTORS.user_code_selector.0,
        user_sp = in(reg) user_stack_pointer,
        user_rip = in(reg) user_code_start.as_u64(),
        options(noreturn)
        );
    }

    unreachable!("iretq failed");
}

#[no_mangle]
pub extern "C" fn test_userspace_routine() {
    unsafe {
        asm!(
            "mov rax, 0xdeadbeef",
            "2:",
            "jmp 2b",
            options(nomem, nostack)
        );
    }
    loop {}
}

#[no_mangle]
pub extern "C" fn syscall_handler() {
    klog!(Debug, "Syscall handler.");
    unsafe {
        asm!("sysretq", options(noreturn));
    }
}
