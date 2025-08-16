use x86_64::instructions::segmentation::CS;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{Segment, SS};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

pub static mut TSS: TaskStateSegment = TaskStateSegment::new();

pub static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
pub static mut SELECTORS: Selectors = Selectors {
    code_selector: SegmentSelector::new(0, x86_64::PrivilegeLevel::Ring0),
    tss_selector: SegmentSelector::new(0, x86_64::PrivilegeLevel::Ring0),
};

pub fn init_tss() -> () {
    let mut tss = TaskStateSegment::new();
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
        const STACK_SIZE: usize = 4096 * 5;
        static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

        let stack_start = VirtAddr::from_ptr(&raw const STACK);
        let stack_end = stack_start + STACK_SIZE as u64;
        stack_end
    };

    unsafe {
        TSS = tss;
    }
}

pub struct Selectors {
    pub code_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

pub fn init_gdt() -> () {
    #[allow(static_mut_refs)]
    unsafe {
        let code_selector = GDT.append(Descriptor::kernel_code_segment());
        let tss_selector = GDT.append(Descriptor::tss_segment(&TSS));

        GDT.load();

        SELECTORS.code_selector = code_selector;
        SELECTORS.tss_selector = tss_selector;

        CS::set_reg(code_selector);
        load_tss(tss_selector);
        SS::set_reg(SegmentSelector { 0: 0 });
    }
}
