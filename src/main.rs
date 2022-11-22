#![feature(
    lang_items,
    custom_test_frameworks,
    abi_x86_interrupt,
    panic_info_message
)]
#![no_std]
#![no_main]
#![allow(dead_code)]

mod gdt;
mod init;
mod interrupts;
mod memory;
mod vga_buffer;

use core::fmt::Write;
use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};
use vga_buffer::{writer, ColourCode};

use x86_64::structures::paging::PageTable;
use x86_64::VirtAddr;

use vga_buffer::{Colour, ColourText};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut display = unsafe { writer::force_lock() };

    let error_colour = ColourCode::new(Colour::White, Colour::Red);

    display.colour = error_colour;

    display.clear_all();

    write!(display, "{info:#?}").unwrap();

    loop {
        x86_64::instructions::hlt();
    }
}

entry_point!(boot_init);

/// Initializes the kernel
#[no_mangle]
fn boot_init(boot_info: &'static BootInfo) -> ! {
    init::init();

    use x86_64::registers::control::Cr3;

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let l4_table = unsafe {
        use memory::active_level_4_table;
        active_level_4_table(phys_mem_offset)
    };

    for (i, entry) in l4_table.iter().enumerate() {
        if !entry.is_unused() {
            println!("L4 Entry {}: {:?}", i, entry);

            // get the physical address from the entry and convert it
            let phys = entry.frame().unwrap().start_address();
            let virt = phys.as_u64() + boot_info.physical_memory_offset;
            let ptr = VirtAddr::new(virt).as_mut_ptr();
            let l3_table: &PageTable = unsafe { &*ptr };

            // print non-empty entries of the level 3 table
            for (i, entry) in l3_table.iter().enumerate() {
                if !entry.is_unused() {
                    println!("  L3 Entry {}: {:?}", i, entry);
                }
            }
        }
    }

    main();

    loop {
        x86_64::instructions::hlt();
    }
}

/// Main runtime
pub fn main() {}
