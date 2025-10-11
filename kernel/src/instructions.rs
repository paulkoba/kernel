use core::arch::asm;

pub const STAR: u32 = 0xC0000081;
pub const LSTAR: u32 = 0xC0000082;
pub const EFER: u32 = 0xC0000080;
pub const FMASK: u32 = 0xC0000084;
pub const KERNEL_GS_BASE: u32 = 0xC0000102;

pub unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    asm!(
        "rdmsr",
        out("eax") low,
        out("edx") high,
        in("ecx") msr,
    );
    ((high as u64) << 32) | (low as u64)
}

pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
    );
}

pub fn nx_enabled() -> bool {
    const IA32_EFER: u32 = 0xC0000080;
    let low: u32;

    unsafe {
        asm!(
        "rdmsr",
        in("ecx") IA32_EFER,
        out("eax") low,
        out("edx") _,
        );
    }

    (low & (1 << 11)) != 0
}
