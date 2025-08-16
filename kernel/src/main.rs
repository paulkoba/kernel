#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use bootloader_api::config::Mapping;
use bootloader_api::{entry_point, BootInfo};
use core::fmt::Write;
use x86_64::structures::paging::{Mapper, Translate};
use x86_64::{PhysAddr, VirtAddr};

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

use crate::logging::{set_log_level, LogLevel};
use crate::serial::SerialPort;

const BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::FixedAddress(0x0000_0001_0000_0000));
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
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
            "Physical memory offset: {:?}",
            VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap())
        );
        let offset_page_table = memory::init(VirtAddr::new(memory_offset));
        klog!(
            Debug,
            "{:?}",
            offset_page_table.translate_addr(VirtAddr::new(0x0000_0001_0000_0123))
        );
    } else {
        klog!(Fatal, "Didn't receive paging info from the bootloader.");
        hcf::hcf();
    }

    hcf::hcf();
}
