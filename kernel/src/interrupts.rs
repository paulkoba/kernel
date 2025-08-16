use crate::interrupt_idx::{PIC_1_OFFSET, PIC_2_OFFSET};
use pic8259::ChainedPics;

pub static mut PICS: ChainedPics = unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) };

pub fn init_interrupts() {
    #[allow(static_mut_refs)]
    unsafe {
        PICS.initialize();
        x86_64::instructions::interrupts::enable();
    }
}
