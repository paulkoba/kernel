#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(optimize_attribute)]
#![allow(dead_code)]
#![allow(static_mut_refs)]
extern crate alloc;

mod allocator;
mod cpuid;
mod freestanding;
mod fs;
mod gdt;
mod hcf;
mod idt;
mod instructions;
mod interrupt_idx;
mod interrupts;
mod logging;
mod memory;
mod panic;
mod serial;
mod syscall;
mod task;
mod time;
mod types;
mod userspace;

use alloc::format;
use alloc::string::String;
use bootloader_api::config::Mapping;
use bootloader_api::{entry_point, BootInfo};
use core::fmt::Write;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::VirtAddr;

use crate::allocator::HeapAllocator;
use crate::cpuid::CpuFeatureEcx;
use crate::logging::{set_log_level, LogLevel};
use crate::memory::{
    init_heap, switch_to_user_page_table, KFrameAllocator, KERNEL_PAGE_TABLE_FRAME,
};
use crate::serial::SerialPort;
use crate::syscall::configure_syscalls;
use crate::task::{create_task, set_current_pid, Task};
use crate::userspace::jump_userspace;

use crate::fs::vfs;
use crate::types::{FMode, Mode};

#[global_allocator]
static mut ALLOCATOR: HeapAllocator = HeapAllocator::new(0, 0);

const BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::FixedAddress(memory::PHYSICAL_MEMORY_OFFSET));
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let mut frame_allocator = unsafe { KFrameAllocator::new(&boot_info.memory_regions) };

    set_log_level(LogLevel::Debug);

    let port = SerialPort::new(0x3F8);
    if port.exists() {
        port.init();
    }

    klog!(Debug, "Serial port test.");

    let cpu_info = cpuid::analyze_cpuid();
    cpuid::log_cpuid_full(&cpu_info);

    if !cpu_info.has_feature_ecx(CpuFeatureEcx::Sse42)
        || !cpu_info.has_feature_ecx(CpuFeatureEcx::Popcnt)
    {
        klog!(Fatal, "Unsupported CPU.");
        hcf::hcf();
    }

    if instructions::nx_enabled() {
        klog!(Debug, "NX bit enabled.");
    } else {
        klog!(Fatal, "NX bit not enabled.");
    }

    time::set_pit_tick_count(0);
    klog!(Debug, "PIT frequency set to {}", time::get_pit_frequency());

    gdt::init_tss();
    klog!(Debug, "Initialized TSS.");
    gdt::init_gdt();
    klog!(Debug, "Initialized GDT.");
    idt::init_idt();
    klog!(Debug, "Initialized IDT.");
    interrupts::init_interrupts();
    klog!(Debug, "Initialized PIC.");

    x86_64::instructions::interrupts::int3();

    let mut offset_page_table: OffsetPageTable;
    if let Some(bootloader_memory_offset) = boot_info.physical_memory_offset.into_option() {
        unsafe {
            KERNEL_PAGE_TABLE_FRAME = bootloader_memory_offset;
        }
        offset_page_table = memory::init(VirtAddr::new(unsafe { KERNEL_PAGE_TABLE_FRAME }));

        init_heap(
            memory::HEAP_START,
            1024 * 1024, // TODO: This should be dynamic and at least ~2% of the total memory.
            &mut offset_page_table,
            &mut frame_allocator,
        )
        .expect("Failed to initialize heap");

        unsafe {
            ALLOCATOR = HeapAllocator::new(memory::HEAP_START, 1024 * 1024);
        }
    } else {
        klog!(Fatal, "Didn't receive paging info from the bootloader.");
        hcf::hcf();
    }

    let string: String = format!("Initialized {}.", "allocator");
    klog!(Debug, "{}", string);

    configure_syscalls();
    let pid = create_task(0, &mut frame_allocator, offset_page_table.phys_offset());
    set_current_pid(pid);
    let task: &mut Task = task::get_current_task().expect("Failed to get current task");
    switch_to_user_page_table(&mut task.page_table);

    vfs::vfs_init();
    unsafe {
        if !vfs::ROOT_DENTRY.is_null() {
            klog!(
                Debug,
                "Mounted RAMFS at: {}",
                vfs::get_full_path(vfs::ROOT_DENTRY)
            );

            // Create /bin directory
            let bin_dir = vfs::mkdir(
                vfs::ROOT_DENTRY,
                "bin",
                Mode::from(0o40777),
                0.into(),
                0.into(),
            );

            if !bin_dir.is_null() {
                klog!(Debug, "Created /bin directory");

                // Embed the init program binary
                const INIT_PROGRAM: &[u8] = include_bytes!("../programs/init.bin");

                // Create /bin/init file
                let init_file_dentry = vfs::create_file(
                    bin_dir,
                    "init",
                    Mode::from(0o100755), // Executable
                    0.into(),
                    0.into(),
                );

                if !init_file_dentry.is_null() {
                    klog!(Debug, "Created /bin/init file");

                    // Write the embedded program to the file
                    if let Some(mut init_file) = vfs::open_file(init_file_dentry, FMode::from(0o2))
                    {
                        let write_result = vfs::write_file(&mut *init_file, INIT_PROGRAM);
                        klog!(Debug, "Wrote {} bytes to /bin/init", write_result);
                        vfs::close_file(init_file);

                        // Read back and print info
                        if let Some(mut init_file) =
                            vfs::open_file(init_file_dentry, FMode::from(0o1))
                        {
                            let dentry_ref = &*init_file_dentry;
                            if !dentry_ref.d_inode.is_null() {
                                let inode_ref = &*dentry_ref.d_inode;
                                let file_size = inode_ref.i_size;
                                klog!(Debug, "File /bin/init length: {} bytes", file_size);

                                // Read the file
                                let mut read_buffer =
                                    alloc::vec::Vec::with_capacity(file_size as usize);
                                read_buffer.resize(file_size as usize, 0);
                                let read_result = vfs::read_file(&mut *init_file, &mut read_buffer);

                                if read_result > 0 {
                                    // Print hexadecimal contents
                                    let mut hex_string = String::new();
                                    for (i, byte) in read_buffer.iter().enumerate() {
                                        if i > 0 && i % 16 == 0 {
                                            hex_string.push('\n');
                                        } else if i > 0 {
                                            hex_string.push(' ');
                                        }
                                        write!(hex_string, "{:02x}", byte).unwrap();
                                    }
                                    klog!(
                                        Debug,
                                        "File /bin/init hexadecimal contents:\n{}",
                                        hex_string
                                    );
                                }
                                vfs::close_file(init_file);
                            }
                        }
                    }
                } else {
                    klog!(Debug, "Failed to create /bin/init file");
                }
            } else {
                klog!(Debug, "Failed to create /bin directory");
            }
        } else {
            klog!(Fatal, "Failed to mount root filesystem");
        }
    }
    jump_userspace(&mut frame_allocator, task);

    hcf::hcf();
}
