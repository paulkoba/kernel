use crate::gdt::SELECTORS;
use crate::instructions::{rdmsr, wrmsr, EFER, FMASK, KERNEL_GS_BASE, LSTAR, STAR};
use crate::task::TrapFrame;
use crate::{klog, logging, LogLevel};
use core::arch::naked_asm;
use core::fmt::Write;

#[repr(align(16))]
struct KernelStack([u8; 16384]);

#[repr(C, align(16))]
struct PerCpu {
    user_rsp: u64,
    kernel_rsp: u64,
    current_task: u64,
    cpu_id: u32,
}

static mut KERNEL_STACK: KernelStack = KernelStack([0; 16384]);
static mut PER_CPU_DATA: PerCpu = PerCpu {
    user_rsp: 0,
    kernel_rsp: 0,
    current_task: 0,
    cpu_id: 0,
};

pub fn configure_syscalls() {
    #[allow(static_mut_refs)]
    unsafe {
        let syscall_handler_addr = syscall_handler as *const () as u64;
        let efer = rdmsr(EFER);

        let syscall_cs_ss_base = (SELECTORS.kernel_code_selector.0 & 0xFFFC) as u32;
        let sysret_cs_ss_base = ((SELECTORS.user_code_selector.0 & 0xFFFC) - 16) as u32;
        let star_high = (syscall_cs_ss_base) | (sysret_cs_ss_base << 16);

        let stack_top = KERNEL_STACK.0.as_ptr() as u64 + 16384;
        PER_CPU_DATA.kernel_rsp = stack_top;
        PER_CPU_DATA.user_rsp = 0;
        PER_CPU_DATA.current_task = 0;
        PER_CPU_DATA.cpu_id = 0;

        let per_cpu_addr = &raw mut PER_CPU_DATA as *mut _ as u64;

        wrmsr(STAR, (star_high as u64) << 32);
        wrmsr(LSTAR, syscall_handler_addr);
        wrmsr(FMASK, 0x0300);
        wrmsr(EFER, efer | 0x1);
        wrmsr(KERNEL_GS_BASE, per_cpu_addr);
    }
}

#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn syscall_handler() {
    naked_asm!(
        "swapgs",
        "mov gs:[0], rsp",
        "mov rsp, gs:[8]",

        "push 0x1b",
        "push gs:[0]",
        "push r11",
        "push 0x23",
        "push rcx",

        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rsi",
        "push rdi",
        "push rbp",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov rdi, rsp",

        "call {handler}",

        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rbp",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",

        "mov rcx, [rsp]",
        "mov r11, [rsp + 16]",
        "mov rsp, [rsp + 24]",

        "swapgs",
        "sysretq",

         handler = sym syscall_dispatch,
    );
}

#[no_mangle]
extern "C" fn syscall_dispatch(frame: &mut TrapFrame) -> u64 {
    klog!(Debug, "Syscall {} from RIP {:#x}", frame.rax, frame.rip);
    klog!(Debug, "{:?}", frame);
    let result = match frame.rax {
        1 => sys_write(frame.rdi, frame.rsi, frame.rdx),
        39 => sys_getpid(),
        60 => sys_exit(frame.rdi),
        _ => u64::MAX,
    };

    frame.rax = result;
    result
}

fn sys_write(fd: u64, buf: u64, count: u64) -> u64 {
    if fd != 1 {
        return u64::MAX;
    }
    klog!(
        Debug,
        "sys_write called with fd={}, buf={:#x}, count={}",
        fd,
        buf,
        count
    );
    count
}

fn sys_getpid() -> u64 {
    123456
}

fn sys_exit(code: u64) -> u64 {
    klog!(Debug, "Process exited with code {}", code);
    42
}
