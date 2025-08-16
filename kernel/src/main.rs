#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use bootloader_api::config::Mapping;
use bootloader_api::{entry_point, BootInfo};
use core::fmt::Write;
use x86_64::structures::paging::{FrameAllocator, Translate};
use x86_64::VirtAddr;

mod allocator;
mod freestanding;
mod gdt;
mod hcf;
mod idt;
mod interrupt_idx;
mod interrupts;
mod logging;
mod memory;
mod panic;
mod serial;
mod time;

use crate::allocator::HeapAllocator;
use crate::logging::{set_log_level, LogLevel};
use crate::memory::{init_heap, BootInfoFrameAllocator};
use crate::serial::SerialPort;

#[global_allocator]
static mut ALLOCATOR: HeapAllocator = HeapAllocator::new(0, 0);

const BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::FixedAddress(0x0000_0001_0000_0000));
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };

    set_log_level(LogLevel::Debug);

    let port = SerialPort::new(0x3F8);
    if port.exists() {
        port.init();
    }

    klog!(Debug, "Serial port test.");

    time::set_pit_tick_count(0);
    klog!(Debug, "PIT frequency set to {}", time::get_pit_frequency());

    gdt::init_tss();
    gdt::init_gdt();
    idt::init_idt();
    interrupts::init_interrupts();

    x86_64::instructions::interrupts::int3();

    klog!(Debug, "Hello from the kernel!");

    if let Some(memory_offset) = boot_info.physical_memory_offset.into_option() {
        klog!(
            Debug,
            "Physical memory offset: {:p}",
            boot_info.physical_memory_offset.into_option().unwrap() as *mut u8
        );
        let mut offset_page_table = memory::init(VirtAddr::new(memory_offset));

        init_heap(
            0x0000_0000_7000_0000,
            1024 * 1024,
            &mut offset_page_table,
            &mut frame_allocator,
        )
        .expect("Failed to initialize heap");
        unsafe {
            ALLOCATOR = HeapAllocator::new(0x0000_0000_7000_0000, 1024 * 1024);
        }
    } else {
        klog!(Fatal, "Didn't receive paging info from the bootloader.");
        hcf::hcf();
    }

    let string: String = format!("Initialized {}", "allocator.");
    klog!(Debug, "{}", string);

    hcf::hcf();
}
