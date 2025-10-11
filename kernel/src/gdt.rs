use x86_64::instructions::segmentation::CS;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{Segment, DS, ES, SS};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const GDT_KERNEL_CODE: usize = 0;
pub const GDT_KERNEL_DATA: usize = 1;
pub const GDT_USER_CODE32: usize = 2;
pub const GDT_USER_DATA: usize = 3;
pub const GDT_USER_CODE: usize = 4;
pub const GDT_TSS: usize = 5;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

pub static mut TSS: TaskStateSegment = TaskStateSegment::new();

pub static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();

pub static mut SELECTORS: Selectors = Selectors {
    kernel_code_selector: SegmentSelector::new(
        GDT_KERNEL_CODE as u16,
        x86_64::PrivilegeLevel::Ring0,
    ),
    kernel_data_selector: SegmentSelector::new(
        GDT_KERNEL_DATA as u16,
        x86_64::PrivilegeLevel::Ring0,
    ),
    user_code32_selector: SegmentSelector::new(
        GDT_USER_CODE32 as u16,
        x86_64::PrivilegeLevel::Ring3,
    ),
    user_code_selector: SegmentSelector::new(GDT_USER_CODE as u16, x86_64::PrivilegeLevel::Ring3),
    user_data_selector: SegmentSelector::new(GDT_USER_DATA as u16, x86_64::PrivilegeLevel::Ring3),
    tss_selector: SegmentSelector::new(GDT_TSS as u16, x86_64::PrivilegeLevel::Ring0),
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

    const KERNEL_STACK_SIZE: usize = 4096 * 5;
    static mut KERNEL_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];
    let kernel_stack_start = VirtAddr::from_ptr(&raw const KERNEL_STACK);
    let kernel_stack_end = kernel_stack_start + KERNEL_STACK_SIZE as u64;
    tss.privilege_stack_table[0] = kernel_stack_end;

    unsafe {
        TSS = tss;
    }
}

pub struct Selectors {
    pub kernel_code_selector: SegmentSelector,
    pub kernel_data_selector: SegmentSelector,
    pub user_code32_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

pub fn init_gdt() -> () {
    #[allow(static_mut_refs)]
    unsafe {
        let kernel_code_selector = GDT.append(Descriptor::kernel_code_segment());
        let kernel_data_selector = GDT.append(Descriptor::kernel_data_segment());
        let user_code32_selector = GDT.append(Descriptor::user_code_segment());
        let user_data_selector = GDT.append(Descriptor::user_data_segment());
        let user_code_selector = GDT.append(Descriptor::user_code_segment());
        let tss_selector = GDT.append(Descriptor::tss_segment(&TSS));

        GDT.load();

        SELECTORS.kernel_code_selector = kernel_code_selector;
        SELECTORS.kernel_data_selector = kernel_data_selector;
        SELECTORS.user_code32_selector = user_code32_selector;
        SELECTORS.user_data_selector = user_data_selector;
        SELECTORS.user_code_selector = user_code_selector;
        SELECTORS.tss_selector = tss_selector;

        CS::set_reg(kernel_code_selector);
        SS::set_reg(kernel_data_selector);
        DS::set_reg(kernel_data_selector);
        ES::set_reg(kernel_data_selector);

        load_tss(tss_selector);
    }
}
