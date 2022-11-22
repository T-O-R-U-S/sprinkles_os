#![feature(
    lang_items,
    custom_test_frameworks,
    abi_x86_interrupt,
    panic_info_message,
    alloc_error_handler
)]

#![no_std]
#![no_main]
#![allow(dead_code)]

extern crate alloc;

mod gdt;
mod init;
mod interrupts;
mod memory;
mod vga_buffer;
mod allocator;

use core::fmt::Write;
use core::panic::PanicInfo;

use alloc::boxed::Box;
use alloc::string::String;
use bootloader::{entry_point, BootInfo};
use vga_buffer::{writer, ColourCode, ColourText};

use vga_buffer::{Colour};
use x86_64::structures::paging::{OffsetPageTable, FrameAllocator, Size4KiB};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut display = unsafe { writer::force_lock() };

    let error_colour = ColourCode::new(Colour::White, Colour::Red);

    display.colour = error_colour;

    display.clear_all();

    write!(display, "{info:#}").unwrap();

    loop {
        x86_64::instructions::hlt();
    }
}

entry_point!(boot_init);

/// Initializes the kernel
#[no_mangle]
fn boot_init(boot_info: &'static BootInfo) -> ! {

    let (frame_allocator, mapper) = unsafe {
        init::init(boot_info)
    };

    main(mapper, frame_allocator);

    loop {
        x86_64::instructions::hlt();
    }
}

/// Main runtime
pub fn main(mut mapper: OffsetPageTable, mut frame_allocator: impl FrameAllocator<Size4KiB>) {
    println!("{}", ColourText::colour(ColourCode(0x3f), "SprinklesOS"));
    println!("Authored by: {}", ColourText::colour(ColourCode(0xdf), "[T-O-R-U-S]"))
}
