use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use core::ops::Sub;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::page_table::PageTable;
use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

pub const PHYSICAL_MEMORY_OFFSET: u64 = 0x0000_1000_0000_0000;
pub const HEAP_START: usize = 0x0000_2000_0000_0000;
pub const USERSPACE_CODE_START: u64 = 0x1000;
pub const USERSPACE_STACK_START: u64 = 0x7FFF_FFFF_F000;
pub const USERSPACE_STACK_SIZE: u64 = 1024 * 1024;
pub const USERSPACE_HEAP_START: u64 = 0x5555_5555_0000;
pub const KERNEL_PML4_INDEX: usize = 256;

pub static mut KERNEL_PAGE_TABLE_FRAME: u64 = 0;

fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

pub fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryRegions) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.kind == MemoryRegionKind::Usable);
        let addr_ranges = usable_regions.map(|r| r.start..r.end);
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

pub fn init_heap(
    heap_start: usize,
    heap_size: u64,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(heap_start as u64);
        let heap_end = heap_start + heap_size - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    Ok(())
}

pub fn create_user_page_table(
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    physical_memory_offset: VirtAddr,
) -> Result<PhysFrame<Size4KiB>, &'static str> {
    let pml4_frame = frame_allocator
        .allocate_frame()
        .ok_or("Failed to allocate frame for PML4")?;

    let pml4_phys = pml4_frame.start_address();
    let pml4_virt = physical_memory_offset + pml4_phys.as_u64();
    let pml4_ptr = pml4_virt.as_mut_ptr::<PageTable>();
    unsafe {
        core::ptr::write_bytes(pml4_ptr, 0, 1);
    }

    let new_p4: &mut PageTable = unsafe { &mut *pml4_ptr };

    let kernel_p4_frame = get_current_page_table();
    let kernel_p4_phys = kernel_p4_frame.start_address();
    let kernel_p4_virt = physical_memory_offset + kernel_p4_phys.as_u64();
    let kernel_p4: &PageTable = unsafe { &*(kernel_p4_virt.as_ptr()) };

    for (i, entry) in kernel_p4.iter().enumerate() {
        let addr = entry.addr();
        if !addr.is_null() && !entry.flags().contains(PageTableFlags::USER_ACCESSIBLE) {
            new_p4[i].set_addr(addr, entry.flags());
        }
    }

    Ok(pml4_frame)
}

pub fn page_table_frame_to_mapper(
    page_table_frame: PhysFrame<Size4KiB>,
    physical_memory_offset: VirtAddr,
) -> OffsetPageTable<'static> {
    let phys_addr = page_table_frame.start_address();
    let virt_addr = physical_memory_offset + phys_addr.as_u64();
    let page_table_ptr = virt_addr.as_mut_ptr::<PageTable>();
    let page_table: &'static mut PageTable = unsafe { &mut *page_table_ptr };

    unsafe { OffsetPageTable::new(page_table, physical_memory_offset) }
}

pub fn create_user_page_table_with_mapper(
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    physical_memory_offset: VirtAddr,
) -> Result<OffsetPageTable<'static>, &'static str> {
    let pml4_frame = create_user_page_table(frame_allocator, physical_memory_offset)?;
    let mapper = page_table_frame_to_mapper(pml4_frame, physical_memory_offset);
    Ok(mapper)
}

pub fn switch_to_user_page_table(page_table: &mut OffsetPageTable) {
    let pml4_virt_addr = page_table.level_4_table() as *const _ as u64;
    let offset = page_table.phys_offset();
    let pml4_phys_addr = VirtAddr::new(pml4_virt_addr).sub(offset);
    let pml4_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(pml4_phys_addr));

    unsafe {
        Cr3::write(pml4_frame, x86_64::registers::control::Cr3Flags::empty());
    }

    x86_64::instructions::tlb::flush_all();
}

pub fn get_current_page_table() -> PhysFrame<Size4KiB> {
    let (frame, _) = Cr3::read();
    frame
}
