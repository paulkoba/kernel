use crate::serial::SerialPort;

use crate::time;
use core::fmt::Write;
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

pub fn serial_write_fmt(args: core::fmt::Arguments) {
    unsafe {
        let _ = writeln!(PORT, "{}", args);
    }
}

pub fn serial_write_fmt_loglevel(log_level: LogLevel, args: core::fmt::Arguments) {
    unsafe {
        if log_level <= KERNEL_LOG_LEVEL {
            serial_write_fmt(args);
        }
    }
}

#[macro_export]
macro_rules! kwriteln {
    ($($arg:tt)*) => {
        $crate::logging::serial_write_fmt(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! klog {
    ($level:ident, $($arg:tt)*) => {
        $crate::logging::serial_write_fmt_loglevel(
            $crate::logging::LogLevel::$level,
            format_args!("[{:.6}] {}", $crate::logging::log_timestamp(), format_args!($($arg)*))
        )
    };
}

pub fn set_log_level(level: LogLevel) {
    unsafe {
        KERNEL_LOG_LEVEL = level;
    }
}
