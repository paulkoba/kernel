#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
extern crate alloc;

use alloc::format;
use alloc::string::String;
use bootloader_api::config::Mapping;
use bootloader_api::{entry_point, BootInfo};
use core::fmt::Write;
use task::Task;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::VirtAddr;

mod allocator;
mod cpuid;
mod freestanding;
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
mod userspace;

use crate::allocator::HeapAllocator;
use crate::cpuid::CpuFeatureEcx;
use crate::logging::{set_log_level, LogLevel};
use crate::memory::{
    init_heap, switch_to_user_page_table, KFrameAllocator, KERNEL_PAGE_TABLE_FRAME,
};
use crate::serial::SerialPort;
use crate::syscall::configure_syscalls;
use crate::userspace::jump_userspace;

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
    let mut task: Task = Task::new(1, &mut frame_allocator, offset_page_table.phys_offset());
    switch_to_user_page_table(&mut task.page_table);
    jump_userspace(&mut frame_allocator, task);
}
