use core::{
    fmt::{self, Display, Write},
    ops::AddAssign, borrow::BorrowMut, array,
};

use alloc::{
    string::{String, ToString},
    vec::{Vec},
};
use lazy_static::lazy_static;
use spin::{Mutex, MutexGuard};
use volatile::Volatile;

/// The width of the VGA buffer
const BUFFER_WIDTH: usize = 80;
/// The height of the VGA buffer
const BUFFER_HEIGHT: usize = 25;

lazy_static! {
    /// The global WRITER that is initialized on OS load
    pub static ref WRITER: Mutex<Writer<BUFFER_WIDTH, BUFFER_HEIGHT, &'static mut Buffer<BUFFER_WIDTH, BUFFER_HEIGHT, Volatile<ScreenChar>>>> = Mutex::new(Writer {
        column_position: ScreenPosition(0),
        row_position: ScreenPosition(0),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer<BUFFER_WIDTH, BUFFER_HEIGHT, Volatile<ScreenChar>>) },
        colour_code: ColourCode::default(),
        lock_colour: false
    });
}

/// Colour codes for VGA text mode display
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

/// A colour code struct.
/// The ColourCode::new method can be used to initialzie new colours using
/// the Colour enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColourCode(pub u8);

/// ColourText is a String that has a VGA text mode colour code attached to it.
/// It implements Display so that you may put it in a println! or format! statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColourText(pub u8, pub String);

/// An abstraction for the on-screen characters.
/// Contains a colour code and a character code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ScreenChar {
    ascii_character: u8,
    colour_code: u8,
}

/// The buffer that contains the references/pointers to each ScreenChar
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Buffer<const X: usize, const Y: usize, T: BorrowMut<Volatile<ScreenChar>>> {
    chars: [[T; X]; Y],
}

/// A value that is clamped to a MAX value to prevent overflow when
/// indexing the writer, to guard against kernel panics. (Undefined/confusing behaviour
/// is preferable to a full system crash.)
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ScreenPosition<const MAX: usize>(pub usize);

/// This trait is used in the Writer struct to help with printing.
pub trait BufWrite {
    /// A character (or a reference to a character) that will be written to
    type Character: BorrowMut<Volatile<ScreenChar>>;
    
    /// Writes a character at a specified position
    fn write_char(&mut self,
        colour_code: ColourCode, 
        ascii_code: u8,
        row: usize,
        col: usize) -> fmt::Result;

    /// Clears the entire screen (using the space character).
    /// `colour` is the colour of the space character that'll clear the screen.
    /// Really, only the foreground colour matters.
    fn clear_screen(
        &mut self,
        colour: ColourCode
    );

    /// Returns a mutable reference to all the characters in the buffer.
    fn char_buf(&mut self) -> Vec<Vec<&mut Self::Character>>;
}

impl<const X: usize, const Y: usize, T: BorrowMut<Volatile<ScreenChar>>> BufWrite for Buffer<X, Y, T> {
    type Character = T;

    fn write_char(&mut self, colour_code: ColourCode, byte: u8, row: usize, col: usize) -> fmt::Result {
        match byte {
            b'\n' => Err(fmt::Error),
            byte => {
                self.chars[row][col].borrow_mut().write(ScreenChar {
                    ascii_character: byte,
                    colour_code: colour_code.into(),
                });

                Ok(())
            }
        }
    }

    fn clear_screen(&mut self, colour: ColourCode) {
        for row in self.chars.iter_mut() {
            for character in row {
                character.borrow_mut().write(ScreenChar {
                    ascii_character: b' ',
                    colour_code: colour.into()
                })
            }
        }
    }

    fn char_buf(&mut self) -> Vec<Vec<&mut T>> {
        let mut buf_ref: Vec<Vec<&mut T>> = vec![];

        for row in self.chars.iter_mut() {
            let mut row_ref = vec![];

            for character_ref in row {
                row_ref.push(character_ref)
            }

            buf_ref.push(row_ref)
        }

        buf_ref
    }
}

impl<const X: usize, const Y: usize, T: BorrowMut<Volatile<ScreenChar>>> BufWrite for &mut Buffer<X, Y, T> {
    type Character = T;

    fn write_char(&mut self, colour_code: ColourCode, byte: u8, row: usize, col: usize) -> fmt::Result {
        match byte {
            b'\n' => Err(fmt::Error),
            byte => {
                self.chars[row][col].borrow_mut().write(ScreenChar {
                    ascii_character: byte,
                    colour_code: colour_code.into(),
                });

                Ok(())
            }
        }
    }

    fn clear_screen(&mut self, colour: ColourCode) {
        for row in self.chars.iter_mut() {
            for character in row {
                character.borrow_mut().write(ScreenChar {
                    ascii_character: b' ',
                    colour_code: colour.into()
                })
            }
        }
    }

    fn char_buf(&mut self) -> Vec<Vec<&mut T>> {
        let mut buf_ref: Vec<Vec<&mut T>> = vec![];

        for row in self.chars.iter_mut() {
            let mut row_ref = vec![];

            for character_ref in row {
                row_ref.push(character_ref)
            }

            buf_ref.push(row_ref)
        }

        buf_ref
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Writer<const X: usize, const Y: usize, Buf: BufWrite> {
    /// The column position in the VGA text buffer
    pub column_position: ScreenPosition<X>,
    /// The row position in the VGA text buffer
    pub row_position: ScreenPosition<Y>,
    /// The buffer that the Writer will write to.
    pub buffer: Buf,
    /// The current colour code of the writer
    pub colour_code: ColourCode,
    /// Whether the colour code is currently locked
    pub lock_colour: bool,
}

/// ColourCode defaults to 0x0f (background black, foreground white)
impl Default for ColourCode {
    fn default() -> Self {
        Self(0x0f)
    }
}

impl Default for ScreenChar {
    fn default() -> Self {
        Self {
            ascii_character: b' ',
            colour_code: Default::default(),
        }
    }
}

impl<'a, const X: usize, const Y: usize> Default for Writer<X, Y, &mut Buffer<X, Y, Volatile<ScreenChar>>> {
    fn default() -> Self {
        Self {
            column_position: Default::default(),
            row_position: Default::default(),
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer<X, Y, Volatile<ScreenChar>>) },
            colour_code: Default::default(),
            lock_colour: true,
        }
    }
}

impl<const MAX: usize> Into<usize> for ScreenPosition<MAX> {
    fn into(self) -> usize {
        self.0
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

impl<const X: usize, const Y: usize, Buf: BufWrite> Writer<X, Y, Buf> {
    /// Writes a character and moves the row and column position forwards to write in the next
    /// available space.
    pub fn write_byte(&mut self, colour_code: ColourCode, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                let row = self.row_position.0;
                let col = self.column_position.0;

                self.buffer.write_char(colour_code, byte, row, col).unwrap_or_else(|_| self.new_line());
                self.column_position += 1;

                // The column position would only get reset if it has overflowed (by reaching the max that
                // the ScreenPosition will allow).
                if self.column_position.0 == 0 {
                    self.new_line()
                }
            }
        }
    }

    /// Write a ColourText to the VGA text buffer.
    pub fn write_colourful(&mut self, s: ColourText) {
        let prev = self.colour_code;
        let mut bytes = s.1.bytes();

        // If the colour is locked, don't change it.
        if !self.lock_colour {
            self.colour_code = s.0.into()
        }

        while let Some(byte) = bytes.next() {
            match byte {
                0x00 => match [bytes.next(), bytes.next()] {
                    [Some(byte_1), Some(byte_2)] => {
                        if !self.lock_colour {
                            self.colour_code = ColourCode(byte_1 + byte_2)
                        }
                    }
                    [Some(byte_1), None] => {
                        if !self.lock_colour {
                            self.write_byte(self.colour_code, byte_1)
                        }
                    }
                    _ => self.write_byte(self.colour_code, 0x00),
                },
                // Printable ASCII range
                0x20..=0x7e | b'\n' => self.write_byte(self.colour_code, byte),
                // If a character is outside the printable ASCII range (i.e DEL, ESC),
                // write a square character in its place to indicate this.
                _ => self.write_byte(self.colour_code, 0xfe),
            }
        }

        self.colour_code = prev;
    }

    /// Same as self.write_colourful(), but it converts `s` into a `ColourText` struct
    pub fn write_string(&mut self, s: &str) {
        self.write_colourful(s.into())
    }

    /// Returns a Writer that can write only within a certain rectangle
    pub fn within_rect<'a, const WIDTH: usize, const HEIGHT: usize>(&'a mut self, offset_x: usize, offset_y: usize) -> Writer<WIDTH, HEIGHT, Buffer<WIDTH, HEIGHT, &'a mut Buf::Character>>
    where &'a mut <Buf as BufWrite>::Character: BorrowMut<Volatile<ScreenChar>>
    {
        let mut char_buf = self.buffer.char_buf();

        let buffer_ref: [[&mut Buf::Character; WIDTH]; HEIGHT] = array::from_fn(|row| 
            array::from_fn(
                |_|  char_buf.get_mut(row + offset_y).unwrap().remove(offset_x)
            )
        );

        return Writer {
            column_position: ScreenPosition(0),
            row_position: ScreenPosition(0),
            buffer: Buffer { chars: buffer_ref },
            colour_code: ColourCode::default(),
            lock_colour: false
        }
    }

    /// Draws a rectangle with the specific character, height, width, and X, Y offset.
    pub fn draw_rect(
        &mut self,
        x: ScreenPosition<X>,
        y: ScreenPosition<Y>,
        width: ScreenPosition<X>,
        height: ScreenPosition<Y>,
        character: u8,
    ) {
        let x = x.0;
        let y = y.0;
        let height = height.0;
        let width = width.0;

        for row in y..y + height {
            for col in x..x+width {
                self.buffer.write_char(self.colour_code, character, row, col).unwrap_or_else(|_| self.new_line());
            }
        }
    }

    /// Draws a newline.
    pub fn new_line(&mut self) {
        self.row_position += 1;
        self.column_position = ScreenPosition(0);

        // The only time that the row_position would be 0 is if the ScreenPosition has overflown its bounds.
        // This means that we've run out of space, and need to clear the buffer.
        // TODO: Move the rest of the text upwards instead of clearing the buffer, discarding the topmost line.
        if self.row_position == ScreenPosition(0) {
            self.clear_all();
        }
    }

    /// Clears the specific row and replaces it with another character
    pub fn clear_row(&mut self, row: usize, screen_char: ScreenChar) {
        for col in 0..X {
            self.buffer.write_char(ColourCode(screen_char.colour_code), screen_char.ascii_character, row, col).unwrap_or_else(|_| self.new_line());
        }
    }

    /// Clears the entire screen.
    pub fn clear_all(&mut self) {
        self.column_position = ScreenPosition(0);
        self.row_position = ScreenPosition(0);

        let blank = self.blank();

        for y in 0..Y {
            self.clear_row(y, blank)
        }
    }

    /// Returns a space character with the Writer's current colour code.
    pub fn blank(&self) -> ScreenChar {
        ScreenChar {
            ascii_character: b' ',
            colour_code: self.colour_code.into(),
        }
    }
}

impl<const X: usize, const Y: usize, Buf: BufWrite> fmt::Write for Writer<X, Y, Buf> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_colourful(ColourText::colour(self.colour_code, s));

        Ok(())
    }
}

pub struct PotentialWriter<'a, T: Write>(Option<MutexGuard<'a, T>>);

impl<T: Write> fmt::Write for PotentialWriter<'_, T> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Some(writer) = &mut self.0 {
            writer.write_str(s)
        } else {
            return Err(fmt::Error);
        }
    }
}

pub mod writer {
    type ScreenWriter = Writer<BUFFER_WIDTH, BUFFER_HEIGHT, &'static mut Buffer<BUFFER_WIDTH, BUFFER_HEIGHT, Volatile<ScreenChar>>>;

    use super::Buffer;
    use super::ColourCode;
    use super::PotentialWriter;
    use super::ScreenChar;
    use super::Writer;
    use super::WRITER;
    use super::{BUFFER_HEIGHT, BUFFER_WIDTH};
    use spin::MutexGuard;
    use volatile::Volatile;

    /// Acquires the global writer.
    pub fn lock<'a>() -> MutexGuard<'a, ScreenWriter> {
        WRITER.lock()
    }

    /// Sets the colour of the global writer.
    pub fn set_colour(colour: ColourCode) -> Result<(), &'static str> {
        let Some(mut writer) = WRITER.try_lock() else {
            return Err("Failed to lock writer to set colour.")
        };

        if !writer.lock_colour {
            return Err("Writer colour is locked");
        }

        writer.colour_code = colour;
        Ok(())
    }

    /// Changes that status of the colour lock of the global writer.
    pub fn lock_colour(set_to: bool) -> Result<(), ()> {
        match WRITER.try_lock() {
            Some(mut writer) => {
                writer.lock_colour = set_to;
                Ok(())
            }
            None => return Err(()),
        }
    }

    /// Returns a PotentialWriter (a new-type wrapper around Writer) that implements
    /// fmt::Write. If the PotentialWriter is none, then it won't write to the VGA
    /// output buffer.
    pub fn maybe<'a>() -> PotentialWriter<'a, ScreenWriter> {
        let Some(writer) = WRITER.try_lock() else {
            return PotentialWriter(None);
        };

        return PotentialWriter(Some(writer));
    }

    /// Attempts to lock the writer. Preferable to a writer::lock because it
    /// evades deadlocks.
    pub fn try_lock<'a>() -> Option<MutexGuard<'a, ScreenWriter>> {
        WRITER.try_lock()
    }

    /// Forcefully unlocks the writer and then locks the now-free writer.
    /// This is unsafe because it might unlock the Mutex while it's still in use.
    pub unsafe fn force_lock<'a>() -> MutexGuard<'a, ScreenWriter> {
        // SAFETY: Might unlock the Writer while it's being used, causing undefined
        // behaviour
        WRITER.force_unlock();
        WRITER.lock()
    }
}
