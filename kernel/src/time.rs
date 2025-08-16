// pic tick count
pub static mut PIT_TICK_COUNT: u64 = 0;

// frequency of the PIC timer in Hz
pub static mut PIT_COUNT: u64 = 0;
pub static PIT_FREQUENCY: u64 = 1193182;

pub fn set_pit_tick_count(count: u64) {
    unsafe {
        PIT_TICK_COUNT = count;

        let mut port = x86_64::instructions::port::Port::new(0x40);
        port.write(count as u8);
        port.write((count >> 8) as u8);
    }
}

pub fn get_pit_tick_count() -> u64 {
    unsafe { PIT_TICK_COUNT }
}

pub fn get_pit_frequency() -> f32 {
    unsafe { PIT_FREQUENCY as f32 / (if PIT_COUNT != 0 { PIT_COUNT } else { 65536 }) as f32 }
}

pub fn time_since_boot() -> f32 {
    get_pit_tick_count() as f32 / get_pit_frequency()
}
