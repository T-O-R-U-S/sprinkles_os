use core::{
    fmt::{self, Display, Write},
    ops::AddAssign,
};

use alloc::{string::{String, ToString}, format, vec::Vec};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColourText(pub u8, pub String);

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

impl Into<[u8; 2]> for ColourCode {
    fn into(self) -> [u8; 2] {
        if self.0 > 0x7f {
            [0x7f, (self.0 - 0x7f)]
        } else {
            [self.0, 0]
        }
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

impl ColourText {
    pub fn colour(colour_code: ColourCode, text: &str) -> Self {
        ColourText(colour_code.0, text.into())
    }

    pub fn text(text: String) -> Self {
        ColourText(0x0f, text.into())
    }
}

impl Display for ColourText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        let mut out = Vec::from([0x00]);
        let colour_escape: [u8; 2] = ColourCode(self.0).into();

        out.extend(colour_escape);
        out.extend(self.1.bytes());
        out.extend([0x00, 0x00, 0x00]);

        let out: String = String::from_utf8_lossy(&out).to_string();

        f.write_str(&out)?;

        Ok(())
    }
}

impl From<&str> for ColourText {
    fn from(value: &str) -> Self {
        ColourText(0x0f, value.into())
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
        let prev = self.colour;
        let mut bytes = s.1.bytes();

        self.colour = s.0.into();

        while let Some(byte) = bytes.next()  {
            match byte {
                0x00 => match [bytes.next(), bytes.next()] {
                    [Some(byte_1), Some(byte_2)] => self.colour = ColourCode(byte_1 + byte_2),
                    [Some(byte_1), None] => self.write_byte(self.colour, byte_1),
                    _ => self.write_byte(self.colour, 0x00)
                }
                // Printable ASCII range
                0x20..=0x7e | b'\n' => self.write_byte(self.colour, byte),
                _ => self.write_byte(self.colour, 0xfe),
            }
        }

        self.colour = prev;
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
        self.write_colourful(ColourText::colour(self.colour, s));

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

pub mod writer {
    use super::ColourCode;
    use super::Writer;
    use super::WRITER;
    use super::{BUFFER_HEIGHT, BUFFER_WIDTH};
    use spin::MutexGuard;

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
