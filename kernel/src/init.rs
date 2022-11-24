use bootloader_api::BootInfo;
use bootloader_api::info::Optional;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::VirtAddr;

use crate::allocator;
use crate::gdt;
use crate::interrupts;
use crate::memory;
use crate::memory::SprinkleFrameAllocator;

pub unsafe fn init(
    boot_info: &'static BootInfo,
) -> (SprinkleFrameAllocator, OffsetPageTable<'static>) {
    gdt::init_gdt();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();

    let Optional::Some(mem_offset) = boot_info.physical_memory_offset else {
        panic!("Memory offset was not set properly");
    };

    let physical_memory_offset = VirtAddr::new(mem_offset);

    let (mut frame_allocator, mut mapper) = (
        SprinkleFrameAllocator::init(&boot_info),
        memory::page_table_init(physical_memory_offset),
    );

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("Failed to initialized the heap.");

    (frame_allocator, mapper)
}
