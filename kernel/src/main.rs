#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::fmt::Write;
use bootloader_api::{entry_point, BootInfo};

mod gdt;
mod idt;
mod serial;
mod logging;
mod memory;
mod panic;

use crate::logging::{set_log_level, LogLevel};
use crate::logging::LogLevel::Debug;
use crate::serial::SerialPort;

const BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let config = bootloader_api::BootloaderConfig::new_default();
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    set_log_level(Debug);
    let port = SerialPort::new(0x3F8);
    if port.exists() {
        port.init();
    }

    klog!(Debug, "Serial port test.");

    gdt::init_tss();
    gdt::init_gdt();
    idt::init_idt();

    x86_64::instructions::interrupts::int3();

    klog!(Debug, "Hello from the kernel!");

    loop {}
}
