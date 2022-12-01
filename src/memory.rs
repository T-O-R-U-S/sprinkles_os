use x86_64::PhysAddr;
use x86_64::structures::paging::{OffsetPageTable, PhysFrame, FrameAllocator, Size4KiB};
use x86_64::{structures::paging::PageTable, VirtAddr};

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

pub struct SprinkleFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize
}

impl SprinkleFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        SprinkleFrameAllocator {
            memory_map,
            next: 0
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);

        let addr_ranges = usable_regions
            .map(|r| r.range.start_addr()..r.range.end_addr());
        
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));

        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for SprinkleFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame 
    }
}

pub unsafe fn page_table_init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

// NOTE: Do not call this function more than once (to avoid aliasing the mutable reference)
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    page_table_ptr.as_mut().unwrap()
}

// x86_64 provides structures for managing PageTables already which are likely to be
// much safer, and they also support huge pages.
//
// pub unsafe fn  translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
//     translate_addr_inner(addr, physical_memory_offset)
// }
//
// fn translate_addr_inner(addr: VirtAddr, mem_offset: VirtAddr) -> Option<PhysAddr> {
//     let (mut frame, _) = Cr3::read();
//
//     let table_indexes = [
//         addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index()
//     ];
//
//     for &index in &table_indexes {
//         let virt = mem_offset + frame.start_address().as_u64();
//         let table_ptr: *const PageTable = virt.as_ptr();
//         let table = unsafe {table_ptr.as_ref().unwrap()};
//
//         let entry = &table[index];
//
//         frame = match entry.frame() {
//             Ok(frame) => frame,
//             Err(FrameError::FrameNotPresent) => return None,
//             Err(FrameError::HugeFrame) => panic!("Huge pages not supported")
//         }
//     }
//
//     Some(frame.start_address() + u64::from(addr.page_offset()))
// }