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
mod runtime;
mod task;

use core::fmt::Write;
use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};
use runtime::{SimpleExecutor, Task};
use vga_buffer::{writer, ColourCode, ColourText};

use vga_buffer::{Colour};
use x86_64::structures::paging::{OffsetPageTable, FrameAllocator, Size4KiB};

use crate::task::keyboard;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut display = unsafe { writer::force_lock() };

    let error_colour = ColourCode::new(Colour::White, Colour::Red);

    display.colour_code = error_colour;

    display.clear_all();

    write!(display, "Kernel panic: {info:#}").expect("Panicked when displaying error message. You're all alone.");

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

    let mut executor = SimpleExecutor::new();
    executor.spawn(Task::new(main(mapper, frame_allocator)));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();

    println!("Done...");

    loop {
        x86_64::instructions::hlt();
    }
}

/// Main runtime
pub async fn main(mut mapper: OffsetPageTable<'static>, mut frame_allocator: impl FrameAllocator<Size4KiB>) {
    println!("{}", ColourText::colour(ColourCode(0x3f), "SprinklesOS"));
    println!("Authored by: {}", ColourText::colour(ColourCode(0xdf), "[T-O-R-U-S]"))
}
