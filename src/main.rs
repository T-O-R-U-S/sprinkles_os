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
mod vga_buffer;

use core::fmt::Write;
use core::panic::PanicInfo;

use alloc::boxed::Box;
use bootloader::{entry_point, BootInfo};
use pc_keyboard::KeyCode;
use runtime::{executor::Executor, Task};
use vga_buffer::{writer, ColourCode, ColourText};

use vga_buffer::Colour;

use crate::task::keyboard;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut display = unsafe { writer::force_lock() };

    let error_colour = ColourCode::new(Colour::White, Colour::Red);

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
#[no_mangle]
fn boot_init(boot_info: &'static BootInfo) -> ! {
    unsafe { init::init(boot_info) };

    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses(
        Box::new(print_key),
        Box::new(print_code),
    )));

    executor.spawn(Task::new(main()));
    executor.run();
}

pub fn print_key(key: char) {
    print!("{key}");
}

pub fn print_code(key: KeyCode) {
    print!("{key:#?}");
}

/// Main runtime
pub async fn main() {
    let mut screen = writer::lock();
    
    writeln!(screen, "{}", ColourText::colour(ColourCode(0x3f), "SprinklesOS"));
    writeln!(screen, 
        "Authored by: {}",
        ColourText::colour(ColourCode(0xdf), "[T-O-R-U-S]")
    );

    let mut sub_screen = screen.within_rect::<30, 20>(5, 5);
}
