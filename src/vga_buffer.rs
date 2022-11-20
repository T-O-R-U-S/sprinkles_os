use core::{ops::{AddAssign}, fmt::{self, Display, Write, Pointer, Arguments}};

use lazy_static::lazy_static;
use volatile::Volatile;
use spin::Mutex;

const BUFFER_WIDTH: usize = 80;
const BUFFER_HEIGHT: usize = 25;

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: ScreenPosition(0),
        row_position: ScreenPosition(0),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Colour {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColourCode(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourText<'a>(pub u8, pub &'a str);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ScreenChar {
    ascii_character: u8,
    colour_code: u8
}

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT]
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ScreenPosition<const MAX: usize>(usize);

impl<const MAX: usize> AddAssign<usize> for ScreenPosition<MAX> {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
        self.0 = self.0 % MAX;
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self {
            column_position: Default::default(),
            row_position: Default::default(),
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer) } 
        }
    }
}

pub struct Writer {
    column_position: ScreenPosition<BUFFER_WIDTH>,
    row_position: ScreenPosition<BUFFER_HEIGHT>,
    buffer: &'static mut Buffer,
}

impl ColourCode {
    pub fn new(foreground: Colour, background: Colour) -> ColourCode {
        ColourCode((background as u8) << 4 | (foreground as u8))
    }
}

impl<'a> ColourText<'a> {
    pub fn colour(colour_code: ColourCode, text: &'a str) -> Self {
        ColourText(colour_code.0, text)
    }

    pub fn text(text: &'a str) -> Self {
        ColourText(0x0f, text)
    }
}

impl<'a> Display for ColourText<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: Impl safe formatting for ColourText
        
        f.write_str(self.1)
    }
}

impl<'a> From<&'a str> for ColourText<'a> {
    fn from(string: &'a str) -> ColourText<'a> {
        ColourText(0x0f, string)
    }
}

impl Writer {
    pub fn write_byte(&mut self, colour_code: u8, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                let row = self.row_position.0;
                let col = self.column_position.0;

                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    colour_code
                });
                self.column_position += 1;

                if self.column_position.0 == 0 {
                    self.new_line()
                }
            }
        }
    }

    pub fn write_colourful(&mut self, s: ColourText) {
        for byte in s.1.bytes() {
            match byte {
                // Printable ASCII range
                0x20..=0x7e | b'\n' => self.write_byte(s.0, byte),
                _ => self.write_byte(s.0, 0xfe)
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        self.write_colourful(s.into())
    }

    pub fn new_line(&mut self) {
        self.row_position += 1;
        self.column_position = ScreenPosition(0);

        self.clear_row(self.row_position.0);
    }

    pub fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            colour_code: 0x00
        };

        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank)
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut colour = ColourCode(0x0f);

        let mut s = s.bytes();

        while let Some(byte) = s.next() {
            match byte {
                0x1b => match s.next() {
                    Some(colour_code) => colour = ColourCode(colour_code),
                    None => return Err(fmt::Error)
                }
                _ => self.write_byte(colour.0, byte)
            }
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    writer::try_lock().unwrap().write_fmt(args).unwrap();
}

pub mod writer {
    use spin::MutexGuard;
    use super::Writer;
    use super::WRITER;
    
    pub fn lock<'a>() -> MutexGuard<'a, Writer> {
        WRITER.lock()
    }

    pub fn try_lock<'a>() -> Option<MutexGuard<'a, Writer>> {
        WRITER.try_lock()
    }

    pub unsafe fn force_lock<'a>() -> MutexGuard<'a, Writer> {
        WRITER.force_unlock();
        WRITER.lock()
    }
}