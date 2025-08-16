use core::arch::x86_64::_rdtsc;
use crate::serial::SerialPort;

use crate::time;

pub static mut PORT: SerialPort = SerialPort::new(0x3F8);
pub static mut KERNEL_LOG_LEVEL: LogLevel = LogLevel::Debug;

#[allow(dead_code)]
#[derive(PartialEq, PartialOrd)]
pub enum LogLevel {
    Off = 0,
    Fatal = 1,
    Error = 2,
    Warn = 3,
    Info = 4,
    Debug = 5,
}

pub fn log_timestamp() -> f32 {
    time::time_since_boot()
}

#[macro_export]
macro_rules! kwriteln {
    ($($arg:tt)*) => {
        unsafe {
            writeln!(logging::PORT, $($arg)*).expect("Failed to write to PORT");
        }
    };
}

#[macro_export]
macro_rules! klog {
    ($level:ident, $($arg:tt)*) => {
        {
            let log_level = LogLevel::$level;
            #[allow(static_mut_refs)]
            unsafe {
                if log_level <= logging::KERNEL_LOG_LEVEL {
                    writeln!(logging::PORT, "[{:.6}] {}", logging::log_timestamp(), format_args!($($arg)*)).expect("Failed to write to PORT");
                }
            }
        }
    };
}

pub fn set_log_level(level: LogLevel) {
    unsafe {
        KERNEL_LOG_LEVEL = level;
    }
}