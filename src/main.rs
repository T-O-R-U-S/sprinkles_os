#![feature(lang_items)]
#![no_std]
#![no_main]

mod vga_buffer;


use core::panic::PanicInfo;

use vga_buffer::{Writer, ColourText};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}


static HELLO: &[u8] = b"Hello world!";

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut writer = Writer::default();

    writer.write_string("Ain't no sunshine");
    
    loop {}
}