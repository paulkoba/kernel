use crate::gdt::SELECTORS;
use crate::instructions::{rdmsr, wrmsr, EFER, FMASK, KERNEL_GS_BASE, LSTAR, STAR};
use crate::{klog, logging, LogLevel};
use core::arch::{asm, naked_asm};
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
#[repr(C)]
pub struct TaskContext {
    pub rsp: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub kernel_stack_top: u64,
    pub kernel_stack_bottom: u64,
    pub trap_frame: *mut TrapFrame,
}

#[repr(C)]
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

pub unsafe fn configure_syscalls() {
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
        "push 0x23",           // CS (user code selector + 3)
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
        "pop rcx",
        "add rsp, 8",
        "pop r11",
        "pop rsp",
        "swapgs",
        "sysretq",

         handler = sym syscall_dispatch,
    );
}

#[no_mangle]
extern "C" fn syscall_dispatch(frame: &mut TrapFrame) -> u64 {
    klog!(Debug, "Syscall {} from RIP {:#x}", frame.rax, frame.rip);

    /*let result = match frame.rax {
        0 => sys_read(frame.rdi, frame.rsi, frame.rdx),
        1 => sys_write(frame.rdi, frame.rsi, frame.rdx),
        // ... more syscalls
        _ => u64::MAX,
    };*/

    //frame.rax = 42;
    42
}
