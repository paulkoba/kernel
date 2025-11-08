use crate::fs::vfs;
use crate::gdt::SELECTORS;
use crate::instructions::{rdmsr, wrmsr, EFER, FMASK, KERNEL_GS_BASE, LSTAR, STAR};
use crate::task::{get_current_task, getpid, getppid, TrapFrame};
use crate::types::FMode;
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
    let task = get_current_task().expect("Failed to get current task");

    task.trap_frame = frame as *mut TrapFrame;

    let result = match frame.rax {
        1 => sys_write(frame.rdi, frame.rsi, frame.rdx),
        2 => sys_open(frame.rdi, frame.rsi, frame.rdx),
        0 => sys_read(frame.rdi, frame.rsi, frame.rdx),
        3 => sys_close(frame.rdi),
        39 => sys_getpid(),
        60 => sys_exit(frame.rdi),
        110 => sys_getppid(),
        _ => u64::MAX,
    };

    frame.rax = result;
    result
}

fn sys_write(fd: u64, buf: u64, count: u64) -> u64 {
    // Handle stdout/stderr (fd 1, 2) - write to kernel log
    if fd == 1 || fd == 2 {
        let slice = unsafe { core::slice::from_raw_parts(buf as *const u8, count as usize) };
        let msg = core::str::from_utf8(slice).unwrap_or("<invalid utf-8>");
        klog!(
            Debug,
            "sys_write called with fd={}, buf=\"{}\", count={}",
            fd,
            msg,
            count
        );
        return count;
    }

    // Handle file descriptors
    let task = match get_current_task() {
        Some(t) => t,
        None => return u64::MAX,
    };

    let file = match task.file_descriptors.get_mut(&fd) {
        Some(f) => f,
        None => return u64::MAX,
    };

    let slice = unsafe { core::slice::from_raw_parts(buf as *const u8, count as usize) };
    let result = vfs::write_file(file.as_mut(), slice);

    if result < 0 {
        u64::MAX
    } else {
        result as u64
    }
}

fn sys_getpid() -> u64 {
    let pid = getpid();
    klog!(Debug, "sys_getpid called, returning pid={}", pid);
    pid
}

fn sys_getppid() -> u64 {
    let ppid = getppid();
    klog!(Debug, "sys_getppid called, returning ppid={}", ppid);
    ppid
}

fn sys_exit(code: u64) -> u64 {
    klog!(Debug, "Process exited with code {}", code);
    42
}

// Read from a file descriptor
// sys_read(fd, buf, count)
fn sys_read(fd: u64, buf: u64, count: u64) -> u64 {
    let task = match get_current_task() {
        Some(t) => t,
        None => return u64::MAX,
    };

    let file = match task.file_descriptors.get_mut(&fd) {
        Some(f) => f,
        None => return u64::MAX,
    };

    let buffer = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, count as usize) };
    let result = vfs::read_file(file.as_mut(), buffer);

    if result < 0 {
        u64::MAX
    } else {
        result as u64
    }
}

// Open a file
// sys_open(pathname, flags, mode)
fn sys_open(pathname: u64, flags: u64, _mode: u64) -> u64 {
    let task = match get_current_task() {
        Some(t) => t,
        None => return u64::MAX,
    };

    // Read the path string from userspace
    // We need to find the null terminator first
    let mut path_len = 0;
    unsafe {
        let mut ptr = pathname as *const u8;
        while *ptr != 0 && path_len < 256 {
            path_len += 1;
            ptr = ptr.add(1);
        }
    }

    if path_len == 0 || path_len >= 256 {
        return u64::MAX;
    }

    let path_slice = unsafe { core::slice::from_raw_parts(pathname as *const u8, path_len) };
    let path_str = match core::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return u64::MAX,
    };

    klog!(
        Debug,
        "sys_open called with path=\"{}\", flags={}",
        path_str,
        flags
    );

    // Resolve the path
    let dentry = vfs::resolve_path(path_str);
    if dentry.is_null() {
        klog!(Debug, "sys_open: path not found");
        return u64::MAX;
    }

    // Convert flags to FMode (simplified - just check for O_RDONLY, O_WRONLY, O_RDWR)
    // O_RDONLY = 0, O_WRONLY = 1, O_RDWR = 2
    let fmode = match flags & 3 {
        0 => FMode::from(0o1), // Read only
        1 => FMode::from(0o2), // Write only
        2 => FMode::from(0o3), // Read/Write
        _ => FMode::from(0o1),
    };

    // Open the file
    let file = match vfs::open_file(dentry, fmode) {
        Some(f) => f,
        None => {
            klog!(Debug, "sys_open: failed to open file");
            return u64::MAX;
        }
    };

    // Allocate a file descriptor
    let fd = task.next_fd;
    task.next_fd += 1;

    // Add to file descriptor table
    task.file_descriptors.insert(fd, file);

    klog!(Debug, "sys_open: opened file with fd={}", fd);
    fd
}

// Close a file descriptor
// sys_close(fd)
fn sys_close(fd: u64) -> u64 {
    let task = match get_current_task() {
        Some(t) => t,
        None => return u64::MAX,
    };

    // Don't allow closing stdin/stdout/stderr
    if fd < 3 {
        return u64::MAX;
    }

    match task.file_descriptors.remove(&fd) {
        Some(file) => {
            vfs::close_file(file);
            klog!(Debug, "sys_close: closed fd={}", fd);
            0
        }
        None => u64::MAX,
    }
}
