use core::{
    fmt::{self, Display, Write},
    ops::AddAssign,
};

use lazy_static::lazy_static;
use spin::{Mutex, MutexGuard};
use volatile::Volatile;
use x86_64::instructions::interrupts;

const BUFFER_WIDTH: usize = 80;
const BUFFER_HEIGHT: usize = 25;

lazy_static! {
    pub static ref WRITER: Mutex<Writer<BUFFER_WIDTH, BUFFER_HEIGHT>> = Mutex::new(Writer {
        column_position: ScreenPosition(0),
        row_position: ScreenPosition(0),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        colour: ColourCode::default()
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
    colour_code: u8,
}

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ScreenPosition<const MAX: usize>(usize);

pub struct Writer<const X: usize, const Y: usize> {
    column_position: ScreenPosition<X>,
    row_position: ScreenPosition<Y>,
    buffer: &'static mut Buffer,
    pub colour: ColourCode,
}

impl Default for ColourCode {
    fn default() -> Self {
        Self(0x0f)
    }
}

impl<const X: usize, const Y: usize> Default for Writer<X, Y> {
    fn default() -> Self {
        Self {
            column_position: Default::default(),
            row_position: Default::default(),
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
            colour: Default::default(),
        }
    }
}

impl<const MAX: usize> AddAssign<usize> for ScreenPosition<MAX> {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
        self.0 = self.0 % MAX;
    }
}

impl ColourCode {
    pub fn new(foreground: Colour, background: Colour) -> ColourCode {
        ColourCode((background as u8) << 4 | (foreground as u8))
    }
}

impl Into<u8> for ColourCode {
    fn into(self) -> u8 {
        self.0
    }
}

impl Into<ColourCode> for u8 {
    fn into(self) -> ColourCode {
        ColourCode(self)
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

impl<'a> From<&'a str> for ColourText<'a> {
    fn from(string: &'a str) -> ColourText<'a> {
        ColourText(0x0f, string)
    }
}

impl<const X: usize, const Y: usize> Writer<X, Y> {
    pub fn write_byte(&mut self, colour_code: ColourCode, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                let row = self.row_position.0;
                let col = self.column_position.0;

                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    colour_code: colour_code.into(),
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
                0x20..=0x7e | b'\n' => self.write_byte(s.0.into(), byte),
                _ => self.write_byte(s.0.into(), 0xfe),
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        self.write_colourful(s.into())
    }

    pub fn new_line(&mut self) {
        self.row_position += 1;
        self.column_position = ScreenPosition(0);

        self.clear_row(self.row_position.0, self.blank());
    }

    pub fn clear_row(&mut self, row: usize, screen_char: ScreenChar) {
        let blank = screen_char;

        for col in 0..X {
            self.buffer.chars[row][col].write(blank)
        }
    }

    pub fn clear_all(&mut self) {
        self.column_position = ScreenPosition(0);
        self.row_position = ScreenPosition(0);

        let blank = self.blank();

        for y in 0..Y {
            self.clear_row(y, blank)
        }
    }

    pub fn blank(&self) -> ScreenChar {
        ScreenChar {
            ascii_character: b' ',
            colour_code: self.colour.into(),
        }
    }
}

impl<const X: usize, const Y: usize> fmt::Write for Writer<X, Y> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let colour = self.colour;

        self.write_colourful(ColourText::colour(colour, s));

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
    interrupts::without_interrupts(|| {
        writer::try_lock()
            .expect("Tried to acquire lock on stdout, failed.")
            .write_fmt(args)
            .unwrap()
    });
}

pub trait WriterDisplay<const X: usize, const Y: usize> {
    fn write(self, writer: MutexGuard<Writer<X, Y>>);
}

impl<const X: usize, const Y: usize> WriterDisplay<X, Y> for ColourText<'_> {
    fn write(self, mut writer: MutexGuard<Writer<X, Y>>) {
        writer.write_colourful(self)
    }
}

impl<const X: usize, const Y: usize, T: Display> WriterDisplay<X, Y> for T {
    fn write(self, mut writer: MutexGuard<Writer<X, Y>>) {
        write!(writer, "{self}")
            .expect("WriterDisplay::print's auto impl for Display types failed");
    }
}

pub mod writer {
    use super::ColourCode;
    use super::Writer;
    use super::WriterDisplay;
    use super::WRITER;
    use super::{BUFFER_HEIGHT, BUFFER_WIDTH};
    use spin::MutexGuard;

    pub fn print(text: impl WriterDisplay<BUFFER_WIDTH, BUFFER_HEIGHT>) {
        let writer = WRITER.lock();

        text.write(writer);
    }

    pub fn lock<'a>() -> MutexGuard<'a, Writer<BUFFER_WIDTH, BUFFER_HEIGHT>> {
        WRITER.lock()
    }

    pub fn set_colour(colour: ColourCode) -> Result<(), &'static str> {
        let Some(mut writer) = WRITER.try_lock() else {
            return Err("Failed to lock writer to set colour.")
        };

        writer.colour = colour;
        Ok(())
    }

    pub fn try_lock<'a>() -> Option<MutexGuard<'a, Writer<BUFFER_WIDTH, BUFFER_HEIGHT>>> {
        WRITER.try_lock()
    }

    pub unsafe fn force_lock<'a>() -> MutexGuard<'a, Writer<BUFFER_WIDTH, BUFFER_HEIGHT>> {
        WRITER.force_unlock();
        WRITER.lock()
    }
}
