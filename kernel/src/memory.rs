use x86_64::registers::control::Cr3;
use x86_64::structures::paging::page_table::{FrameError, PageTable};
use x86_64::structures::paging::PageTableFlags as Flags;
use x86_64::{PhysAddr, VirtAddr};

pub unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

// TODO: no idea how this works
pub fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];
    let mut frame = level_4_table_frame;

    for (level, &index) in table_indexes.iter().enumerate() {
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };

        let entry = &table[index];

        if !entry.flags().contains(Flags::PRESENT) {
            return None;
        }

        if entry.flags().contains(Flags::HUGE_PAGE) {
            return match level {
                1 => {
                    let offset = addr.as_u64() & 0x3fff_ffff;
                    Some(entry.addr() + offset)
                }
                2 => {
                    let offset = addr.as_u64() & 0x1f_ffff;
                    Some(entry.addr() + offset)
                }
                _ => panic!("huge pages only supported at P2 or P3 levels"),
            };
        }

        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("unexpected huge page error"),
        };
    }

    Some(frame.start_address() + u64::from(addr.page_offset()))
}
