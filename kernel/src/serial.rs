use core::fmt::{Result, Write};

pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    pub const fn new(port: u16) -> SerialPort {
        SerialPort { port }
    }

    pub fn init(&self) {
        unsafe {
            outb(self.port + 1, 0x00); // Disable all interrupts
            outb(self.port + 3, 0x80); // Enable DLAB (set baud rate divisor)
            outb(self.port + 0, 0x03); // Set divisor to 3 (lo byte) 38400 baud
            outb(self.port + 1, 0x00); // (hi byte)
            outb(self.port + 3, 0x03); // 8 bits, no parity, one stop bit
            outb(self.port + 2, 0xC7); // Enable FIFO, clear them, with 14-byte threshold
            outb(self.port + 4, 0x0B); // IRQs enabled, RTS/DSR set
        }
    }

    // Check if the serial port exists using scratch register
    pub fn exists(&self) -> bool {
        unsafe {
            let original = inb(self.port + 7);
            outb(self.port + 7, 0x55);
            if inb(self.port + 7) == 0x55 {
                outb(self.port + 7, original);
                true
            } else {
                false
            }
        }
    }

    fn write_byte(&self, byte: u8) {
        unsafe {
            while (inb(self.port + 5) & 0x20) == 0 {}
            outb(self.port, byte);
        }
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> Result {
        for b in s.bytes() {
            self.write_byte(b);
        }
        Ok(())
    }
}

unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val);
}

unsafe fn inb(port: u16) -> u8 {
    let mut ret: u8;
    core::arch::asm!("in al, dx", out("al") ret, in("dx") port);
    ret
}
