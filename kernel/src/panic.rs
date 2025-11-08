// makes clion error highlighting sad otherwise :-(
#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
use crate::klog;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    klog!(Fatal, "Kernel panic: {}", _info);
    loop {}
}
