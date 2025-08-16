pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
#[allow(dead_code)]
pub enum InterruptIndex {
    Timer    = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET + 1,
    Cascade  = PIC_1_OFFSET + 2,
    Com2     = PIC_1_OFFSET + 3,
    Com1     = PIC_1_OFFSET + 4,
    Lpt2     = PIC_1_OFFSET + 5,
    Floppy   = PIC_1_OFFSET + 6,
    Lpt1     = PIC_1_OFFSET + 7,
}

impl InterruptIndex {
    pub(crate) fn as_u8(self) -> u8 {
        self as u8
    }
}
