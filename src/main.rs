#![feature(
    lang_items,
    custom_test_frameworks,
    abi_x86_interrupt,
    panic_info_message,
    alloc_error_handler,
    associated_type_bounds,
)]
#![no_std]
#![no_main]
#![allow(dead_code)]

#[macro_use(vec)]
extern crate alloc;

mod allocator;
mod gdt;
mod init;
mod interrupts;
mod memory;
mod runtime;
mod task;
pub mod vga_buffer;
pub mod fs;

use core::fmt::Write;
use core::panic::PanicInfo;

use alloc::boxed::Box;
use bootloader::{entry_point, BootInfo};
use pc_keyboard::{DecodedKey};
use runtime::{executor::Executor, Task};
use vga_buffer::{global_writer, ColourCode, ColourText};

use vga_buffer::Colour::*;

use crate::task::keyboard;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut display = unsafe { global_writer::force_lock() };

    let error_colour = ColourCode::new(White, Red);

    display.colour_code = error_colour;

    display.clear_all();

    write!(display, "Kernel panic: {info:#}")
        .expect("Panicked when displaying error message. You're all alone.");

    loop {
        x86_64::instructions::hlt();
    }
}

entry_point!(boot_init);

/// Initializes the kernel
fn boot_init(boot_info: &'static BootInfo) -> ! {
    unsafe { init::init(boot_info) };

    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::handle_keypresses(
        Box::new(print_key),
    )));

    executor.spawn(Task::new(main()));
    executor.run();
}

// Placeholder functions to print each pressed key until a true handling system can be implemented
pub fn print_key(key: DecodedKey) {
    match key {
        DecodedKey::RawKey(raw) => write!(global_writer::maybe(), "{raw:?}").ok(),
        DecodedKey::Unicode(character) => write!(global_writer::maybe(), "{character}").ok(),
    };
}

/// Main runtime
pub async fn main() {
    let mut screen = global_writer::lock();

    writeln!(screen, "{}", ColourText::colour(ColourCode(0x3f), "SprinklesOS")).ok();
    writeln!(screen, 
        "Authored by: {}",
        ColourText::colour(ColourCode(0xdf), "[T-O-R-U-S]")
    ).ok();
}

