#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use bootloader_api::config::Mapping;
use bootloader_api::{entry_point, BootInfo};
use core::fmt::Write;
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
use crate::memory::{active_level_4_table, translate_addr_inner};
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
        let phys_mem_offset = VirtAddr::new(memory_offset);
        let l4_table = unsafe { active_level_4_table(phys_mem_offset) };

        for (i, entry) in l4_table.iter().enumerate() {
            if !entry.is_unused() {
                klog!(Info, "L4 Entry {}: {:?}", i, entry);
            }
        }
    } else {
        klog!(Fatal, "Didn't receive paging info from the bootloader.");
        hcf::hcf();
    }

    klog!(
        Debug,
        "Translation test: address 0x0000_0001_0000_0123 translates to {:?}",
        PhysAddr::new(
            translate_addr_inner(
                VirtAddr::new(0x0000_0001_0000_0123),
                VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap())
            )
            .unwrap_or(PhysAddr::new(42))
            .as_u64()
        )
    );

    hcf::hcf();
}
