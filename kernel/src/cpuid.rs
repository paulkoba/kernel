use crate::klog;
use core::arch::x86_64::__cpuid_count;

#[derive(Debug)]
pub struct CpuInfo {
    pub vendor: [u8; 12],
    pub max_standard_leaf: u32,
    pub max_extended_leaf: u32,
    pub features_ecx: u32,
    pub features_edx: u32,
    pub processor_name: [u8; 48],
    pub cores_per_package: u32,
    pub threads_per_core: u32,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum CpuFeatureEcx {
    Sse3 = 1 << 0,
    Pclmul = 1 << 1,
    Dtes64 = 1 << 2,
    Monitor = 1 << 3,
    DsCpl = 1 << 4,
    Vmx = 1 << 5,
    Smx = 1 << 6,
    Est = 1 << 7,
    Tm2 = 1 << 8,
    Ssse3 = 1 << 9,
    Cid = 1 << 10,
    Sdbg = 1 << 11,
    Fma = 1 << 12,
    Cx16 = 1 << 13,
    Xtpr = 1 << 14,
    Pdcm = 1 << 15,
    Pcid = 1 << 17,
    Dca = 1 << 18,
    Sse41 = 1 << 19,
    Sse42 = 1 << 20,
    X2apic = 1 << 21,
    Movbe = 1 << 22,
    Popcnt = 1 << 23,
    Tsc = 1 << 24,
    Aes = 1 << 25,
    Xsave = 1 << 26,
    Osxsave = 1 << 27,
    Avx = 1 << 28,
    F16c = 1 << 29,
    Rdrand = 1 << 30,
    Hypervisor = 1 << 31,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum CpuFeatureEdx {
    Fpu = 1 << 0,
    Vme = 1 << 1,
    De = 1 << 2,
    Pse = 1 << 3,
    Tsc = 1 << 4,
    Msr = 1 << 5,
    Pae = 1 << 6,
    Mce = 1 << 7,
    Cx8 = 1 << 8,
    Apic = 1 << 9,
    Sep = 1 << 11,
    Mtrr = 1 << 12,
    Pge = 1 << 13,
    Mca = 1 << 14,
    Cmov = 1 << 15,
    Pat = 1 << 16,
    Pse36 = 1 << 17,
    Psn = 1 << 18,
    Clflush = 1 << 19,
    Ds = 1 << 21,
    Acpi = 1 << 22,
    Mmx = 1 << 23,
    Fxsr = 1 << 24,
    Sse = 1 << 25,
    Sse2 = 1 << 26,
    Ss = 1 << 27,
    Htt = 1 << 28,
    Tm = 1 << 29,
    Ia64 = 1 << 30,
    Pbe = 1 << 31,
}

impl CpuInfo {
    pub fn has_feature_ecx(&self, feature: CpuFeatureEcx) -> bool {
        (self.features_ecx & (feature as u32)) != 0
    }

    #[allow(dead_code)]
    pub fn has_feature_edx(&self, feature: CpuFeatureEdx) -> bool {
        (self.features_edx & (feature as u32)) != 0
    }
}

pub fn analyze_cpuid() -> CpuInfo {
    unsafe {
        let mut info = CpuInfo {
            vendor: [0; 12],
            max_standard_leaf: 0,
            max_extended_leaf: 0,
            features_ecx: 0,
            features_edx: 0,
            processor_name: [0; 48],
            cores_per_package: 0,
            threads_per_core: 0,
        };

        let leaf_0 = __cpuid_count(0, 0);
        info.max_standard_leaf = leaf_0.eax;

        let vendor_parts = [leaf_0.ebx, leaf_0.edx, leaf_0.ecx];
        for (i, &part) in vendor_parts.iter().enumerate() {
            info.vendor[i * 4..(i + 1) * 4].copy_from_slice(&part.to_le_bytes());
        }

        if info.max_standard_leaf >= 1 {
            let leaf_1 = __cpuid_count(1, 0);
            info.features_ecx = leaf_1.ecx;
            info.features_edx = leaf_1.edx;
            // TODO: This one is just straight out wrong, probably needs to be fixed
            info.threads_per_core = (leaf_1.ebx >> 16) & 0xFF;
            if info.threads_per_core == 0 {
                info.threads_per_core = 1;
            }
        }

        let ext_leaf_0 = __cpuid_count(0x80000000, 0);
        info.max_extended_leaf = ext_leaf_0.eax;

        if info.max_extended_leaf >= 0x80000004 {
            for (i, leaf) in [0x80000002, 0x80000003, 0x80000004].iter().enumerate() {
                let result = __cpuid_count(*leaf, 0);
                let registers = [result.eax, result.ebx, result.ecx, result.edx];
                for (j, &reg) in registers.iter().enumerate() {
                    info.processor_name[i * 16 + j * 4..i * 16 + (j + 1) * 4]
                        .copy_from_slice(&reg.to_le_bytes());
                }
            }
        }

        if info.max_standard_leaf >= 4 {
            let leaf_4 = __cpuid_count(4, 0);
            info.cores_per_package = ((leaf_4.eax >> 26) & 0x3F) + 1;
        } else if info.max_extended_leaf >= 0x80000008 {
            let leaf_80000008 = __cpuid_count(0x80000008, 0);
            info.cores_per_package = (leaf_80000008.ecx & 0xFF) + 1;
        }

        info
    }
}

pub fn log_cpuid_full(info: &CpuInfo) {
    klog!(
        Debug,
        "CPU Vendor       : {:?}",
        core::str::from_utf8(&info.vendor).unwrap_or("Unknown")
    );
    klog!(Debug, "Max Standard Leaf: {}", info.max_standard_leaf);
    klog!(Debug, "Max Extended Leaf: {}", info.max_extended_leaf);
    klog!(Debug, "Features ECX     : {:#010x}", info.features_ecx);
    klog!(Debug, "Features EDX     : {:#010x}", info.features_edx);
    klog!(
        Debug,
        "Processor Name   : {:?}",
        core::str::from_utf8(&info.processor_name).unwrap_or("Unknown")
    );
    klog!(Debug, "Cores per Package: {}", info.cores_per_package);
    klog!(Debug, "Total cores      : {}", info.threads_per_core);
}
