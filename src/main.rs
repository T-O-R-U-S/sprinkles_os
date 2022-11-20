#![feature(lang_items)]
#![no_std]
#![no_main]

mod vga_buffer;
use core::panic::PanicInfo;

use vga_buffer::{ColourText, ColourCode};

use crate::vga_buffer::Colour;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!(r#"
SprinklesOS
Author: [{}]
"#,
ColourText::colour(ColourCode::new(Colour::White, Colour::Pink), "T-O-R-U-S")
);

    loop {}
}