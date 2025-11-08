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

            // Test 1: Basic write and read
            klog!(Debug, "--- Test 1: Basic Write/Read ---");
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

                // Write test data
                let mut file = match vfs::open_file(test_file_dentry, FMode::from(0o2)) {
                    Some(f) => f,
                    None => {
                        klog!(Fatal, "Failed to open file for writing");
                        hcf::hcf();
                    }
                };

                let test_data = b"Hello, RAMFS! This is a test of file read/write operations.\n";
                let write_result = vfs::write_file(&mut *file, test_data);
                klog!(Debug, "Wrote {} bytes to file", write_result);

                let dentry_ref = &*test_file_dentry;
                if !dentry_ref.d_inode.is_null() {
                    let inode_ref = &*dentry_ref.d_inode;
                    klog!(Debug, "File size after write: {} bytes", inode_ref.i_size);
                }

                vfs::close_file(file);

                // Read the data back
                let mut file = match vfs::open_file(test_file_dentry, FMode::from(0o1)) {
                    Some(f) => f,
                    None => {
                        klog!(Fatal, "Failed to open file for reading");
                        hcf::hcf();
                    }
                };

                let mut read_buffer = [0u8; 128];
                let read_result = vfs::read_file(&mut *file, &mut read_buffer);
                klog!(Debug, "Read {} bytes from file", read_result);

                if read_result > 0 {
                    let read_data = &read_buffer[..read_result as usize];
                    if read_data == test_data {
                        klog!(Debug, "✓ Test 1 PASSED: Basic read/write works");
                        if let Ok(content) = core::str::from_utf8(read_data) {
                            klog!(Debug, "Content: \"{}\"", content.trim_end());
                        }
                    } else {
                        klog!(Fatal, "✗ Test 1 FAILED: Data mismatch");
                    }
                }

                vfs::close_file(file);

                // Test 2: Multiple writes (append mode)
                klog!(Debug, "--- Test 2: Multiple Writes (Append) ---");
                let mut file = match vfs::open_file(test_file_dentry, FMode::from(0o2)) {
                    Some(f) => f,
                    None => {
                        klog!(Fatal, "Failed to open file for writing");
                        hcf::hcf();
                    }
                };

                // Seek to end (by reading to EOF or checking size)
                let dentry_ref = &*test_file_dentry;
                if !dentry_ref.d_inode.is_null() {
                    let inode_ref = &*dentry_ref.d_inode;
                    file.f_pos = inode_ref.i_size; // Seek to end
                }

                let append_data = b"Appended line 1\nAppended line 2\n";
                let write_result2 = vfs::write_file(&mut *file, append_data);
                klog!(Debug, "Appended {} bytes", write_result2);

                if !dentry_ref.d_inode.is_null() {
                    let inode_ref = &*dentry_ref.d_inode;
                    klog!(Debug, "File size after append: {} bytes", inode_ref.i_size);
                }

                vfs::close_file(file);

                // Read entire file
                let mut file = match vfs::open_file(test_file_dentry, FMode::from(0o1)) {
                    Some(f) => f,
                    None => {
                        klog!(Fatal, "Failed to open file for reading");
                        hcf::hcf();
                    }
                };

                let mut read_buffer2 = [0u8; 256];
                let read_result2 = vfs::read_file(&mut *file, &mut read_buffer2);
                klog!(Debug, "Read {} bytes (full file)", read_result2);

                let expected_full = [test_data.as_slice(), append_data.as_slice()].concat();
                let read_full = &read_buffer2[..read_result2 as usize];
                if read_full == expected_full.as_slice() {
                    klog!(Debug, "✓ Test 2 PASSED: Multiple writes work correctly");
                } else {
                    klog!(Debug, "✗ Test 2 FAILED: Append data mismatch");
                }

                vfs::close_file(file);

                // Test 3: Reading from specific positions
                klog!(Debug, "--- Test 3: Reading from Specific Positions ---");
                let mut file = match vfs::open_file(test_file_dentry, FMode::from(0o1)) {
                    Some(f) => f,
                    None => {
                        klog!(Fatal, "Failed to open file for reading");
                        hcf::hcf();
                    }
                };

                // Read from position 7 (skip "Hello,")
                file.f_pos = 7;
                let mut read_buffer3 = [0u8; 32];
                let read_result3 = vfs::read_file(&mut *file, &mut read_buffer3);
                klog!(Debug, "Read {} bytes from position 7", read_result3);

                if read_result3 > 0 {
                    let read_len = read_result3 as usize;
                    let read_partial = &read_buffer3[..read_len];
                    if let Ok(content) = core::str::from_utf8(read_partial) {
                        klog!(Debug, "Content from pos 7: \"{}\"", content.trim_end());
                    }
                    // Check if it matches expected substring (from the full file content)
                    let full_content = [test_data.as_slice(), append_data.as_slice()].concat();
                    if full_content.len() > 7 {
                        let expected_substr = &full_content[7..];
                        let min_len = read_len.min(expected_substr.len());
                        if read_partial[..min_len] == expected_substr[..min_len] {
                            klog!(
                                Debug,
                                "✓ Test 3 PASSED: Reading from specific position works"
                            );
                        } else {
                            klog!(Debug, "✗ Test 3 FAILED: Position read mismatch");
                        }
                    }
                }

                vfs::close_file(file);

                // Test 4: Writing at specific positions (overwrite)
                klog!(Debug, "--- Test 4: Writing at Specific Positions ---");
                let mut file = match vfs::open_file(test_file_dentry, FMode::from(0o2)) {
                    Some(f) => f,
                    None => {
                        klog!(Fatal, "Failed to open file for writing");
                        hcf::hcf();
                    }
                };

                // Write at position 0 (overwrite beginning)
                file.f_pos = 0;
                let overwrite_data = b"Hi!";
                let write_result3 = vfs::write_file(&mut *file, overwrite_data);
                klog!(Debug, "Wrote {} bytes at position 0", write_result3);

                vfs::close_file(file);

                // Read and verify
                let mut file = match vfs::open_file(test_file_dentry, FMode::from(0o1)) {
                    Some(f) => f,
                    None => {
                        klog!(Fatal, "Failed to open file for reading");
                        hcf::hcf();
                    }
                };

                let mut read_buffer4 = [0u8; 8];
                let read_result4 = vfs::read_file(&mut *file, &mut read_buffer4);
                if read_result4 >= 3 {
                    let read_start = &read_buffer4[..3];
                    if read_start == overwrite_data {
                        klog!(Debug, "✓ Test 4 PASSED: Writing at specific position works");
                    } else {
                        klog!(Debug, "✗ Test 4 FAILED: Overwrite mismatch");
                    }
                }

                vfs::close_file(file);

                // Test 5: Multiple files
                klog!(Debug, "--- Test 5: Multiple Files ---");
                let file2_dentry = vfs::create_file(
                    vfs::ROOT_DENTRY,
                    "test2.txt",
                    Mode::from(0o100644),
                    0.into(),
                    0.into(),
                );

                if !file2_dentry.is_null() {
                    let mut file2 = match vfs::open_file(file2_dentry, FMode::from(0o2)) {
                        Some(f) => f,
                        None => {
                            klog!(Fatal, "Failed to open file2 for writing");
                            hcf::hcf();
                        }
                    };

                    let file2_data = b"Second file content\n";
                    let write_result4 = vfs::write_file(&mut *file2, file2_data);
                    klog!(Debug, "Wrote {} bytes to file2", write_result4);

                    vfs::close_file(file2);

                    // Read from both files
                    let mut file1 = match vfs::open_file(test_file_dentry, FMode::from(0o1)) {
                        Some(f) => f,
                        None => {
                            klog!(Fatal, "Failed to open file1 for reading");
                            hcf::hcf();
                        }
                    };

                    let mut file2 = match vfs::open_file(file2_dentry, FMode::from(0o1)) {
                        Some(f) => f,
                        None => {
                            klog!(Fatal, "Failed to open file2 for reading");
                            hcf::hcf();
                        }
                    };

                    let mut buf1 = [0u8; 64];
                    let mut buf2 = [0u8; 64];
                    let read1 = vfs::read_file(&mut *file1, &mut buf1);
                    let read2 = vfs::read_file(&mut *file2, &mut buf2);

                    klog!(
                        Debug,
                        "Read {} bytes from file1, {} bytes from file2",
                        read1,
                        read2
                    );

                    if read1 > 0 && read2 > 0 {
                        let data1 = &buf1[..read1 as usize];
                        let data2 = &buf2[..read2 as usize];
                        if data2 == file2_data {
                            klog!(Debug, "✓ Test 5 PASSED: Multiple files work independently");
                        } else {
                            klog!(Debug, "✗ Test 5 FAILED: File2 data mismatch");
                        }
                    }

                    vfs::close_file(file1);
                    vfs::close_file(file2);
                }

                klog!(Debug, "=== File Read/Write Showcase Complete ===");
            } else {
                klog!(Fatal, "Failed to create test file");
            }

            // Create a dummy file for userspace to access
            klog!(Debug, "Creating dummy file for userspace...");
            let dummy_file_dentry = vfs::create_file(
                vfs::ROOT_DENTRY,
                "dummy.txt",
                Mode::from(0o100644),
                0.into(),
                0.into(),
            );

            if !dummy_file_dentry.is_null() {
                klog!(
                    Debug,
                    "Created dummy file at: {}",
                    vfs::get_full_path(dummy_file_dentry)
                );

                // Write some initial content to the dummy file
                if let Some(mut dummy_file) = vfs::open_file(dummy_file_dentry, FMode::from(0o2)) {
                    let dummy_content =
                        b"Hello from kernel! This is a dummy file for userspace testing.\n";
                    let write_result = vfs::write_file(&mut *dummy_file, dummy_content);
                    klog!(Debug, "Wrote {} bytes to dummy file", write_result);
                    vfs::close_file(dummy_file);
                } else {
                    klog!(Debug, "Failed to open dummy file for writing");
                }
            } else {
                klog!(Debug, "Failed to create dummy file");
            }
        } else {
            klog!(Fatal, "Failed to mount root filesystem");
        }
    }
    jump_userspace(&mut frame_allocator, task);

    hcf::hcf();
}
