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

use crate::fs::ramfs;
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
            let a = vfs::mkdir(
                vfs::ROOT_DENTRY,
                "a",
                Mode::from(0o40777),
                0.into(),
                0.into(),
            );
            if !a.is_null() {
                klog!(Debug, "Created directory at: {}", vfs::get_full_path(a));
                let b = vfs::mkdir(a, "b", Mode::from(0o40777), 0.into(), 0.into());
                if !b.is_null() {
                    klog!(Debug, "Created directory at: {}", vfs::get_full_path(b));
                }
            }

            // Showcase: File read/write operations
            klog!(Debug, "=== File Read/Write Showcase ===");

            // Create a test file
            let test_file_dentry = vfs::create_file(
                vfs::ROOT_DENTRY,
                "test.txt",
                Mode::from(0o100644), // Regular file with rw-r--r-- permissions
                0.into(),
                0.into(),
            );

            if !test_file_dentry.is_null() {
                klog!(
                    Debug,
                    "Created test file at: {}",
                    vfs::get_full_path(test_file_dentry)
                );

                // Open the file for writing
                let mut file = match vfs::open_file(test_file_dentry, FMode::from(0o2)) {
                    Some(f) => {
                        klog!(Debug, "Opened file for writing");
                        f
                    }
                    None => {
                        klog!(Fatal, "Failed to open file for writing");
                        hcf::hcf();
                    }
                };

                // Write test data
                let test_data = b"Hello, RAMFS! This is a test of file read/write operations.\n";
                let write_result = vfs::write_file(&mut *file, test_data);
                klog!(Debug, "Wrote {} bytes to file", write_result);

                if write_result > 0 {
                    // Get file size from inode
                    let dentry_ref = &*test_file_dentry;
                    if !dentry_ref.d_inode.is_null() {
                        let inode_ref = &*dentry_ref.d_inode;
                        klog!(Debug, "File size after write: {} bytes", inode_ref.i_size);
                    }
                }

                // Close the file
                vfs::close_file(file);
                klog!(Debug, "Closed file after write");

                // Reopen the file for reading
                let mut file = match vfs::open_file(test_file_dentry, FMode::from(0o1)) {
                    Some(f) => {
                        klog!(Debug, "Opened file for reading");
                        f
                    }
                    None => {
                        klog!(Fatal, "Failed to open file for reading");
                        hcf::hcf();
                    }
                };

                // Read the data back
                let mut read_buffer = [0u8; 128];
                let read_result = vfs::read_file(&mut *file, &mut read_buffer);
                klog!(Debug, "Read {} bytes from file", read_result);

                if read_result > 0 {
                    // Verify the data matches
                    let read_data = &read_buffer[..read_result as usize];
                    if read_data == test_data {
                        klog!(
                            Debug,
                            "✓ Data verification: SUCCESS - Read data matches written data"
                        );
                        // Try to print the content (may contain non-printable chars)
                        if let Ok(content) = core::str::from_utf8(read_data) {
                            klog!(Debug, "File content: \"{}\"", content.trim_end());
                        }
                    } else {
                        klog!(Fatal, "✗ Data verification: FAILED - Data mismatch");
                        klog!(Debug, "Expected: {:?}", test_data);
                        klog!(Debug, "Got: {:?}", read_data);
                    }

                    // Try reading again from the beginning
                    file.f_pos = 0;
                    let mut read_buffer2 = [0u8; 128];
                    let read_result2 = vfs::read_file(&mut *file, &mut read_buffer2);
                    klog!(Debug, "Second read (from start): {} bytes", read_result2);

                    if read_result2 == read_result {
                        let read_data2 = &read_buffer2[..read_result2 as usize];
                        if read_data2 == test_data {
                            klog!(
                                Debug,
                                "✓ Multiple reads work correctly - data matches on second read"
                            );
                        } else {
                            klog!(Debug, "✗ Second read data mismatch");
                        }
                    }
                } else {
                    klog!(Fatal, "Failed to read from file");
                }

                // Close the file
                vfs::close_file(file);
                klog!(Debug, "Closed file after read");

                klog!(Debug, "=== File Read/Write Showcase Complete ===");
            } else {
                klog!(Fatal, "Failed to create test file");
            }
        } else {
            klog!(Fatal, "Failed to mount root filesystem");
        }
    }
    jump_userspace(&mut frame_allocator, task);

    hcf::hcf();
}
