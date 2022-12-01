use core::{
    pin::Pin,
    task::{Context, Poll},
};
use core::fmt::Write;

use conquer_once::spin::OnceCell;
use crossbeam::queue::ArrayQueue;
use futures_util::Stream;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

use futures_util::task::AtomicWaker;

use crate::{
    vga_buffer::{Colour, ColourCode, ColourText, global_writer},
};

static WAKER: AtomicWaker = AtomicWaker::new();

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("Keyboard input queue not initialized");
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

pub(crate) fn add_scancode(scancode: u8) {
    let warn = ColourText::colour(ColourCode::new(Colour::Black, Colour::Yellow), "WARNING:");

    let queue = SCANCODE_QUEUE.try_get().expect("Input queue uninitialized");

    if let Err(_) = queue.push(scancode) {
        writeln!(global_writer::maybe(), "{warn}: scancode queue full; dropping keyboard input").ok();
    } else {
        WAKER.wake()
    }
}

use futures_util::stream::StreamExt;
use pc_keyboard::{layouts, DecodedKey, HandleControl, KeyCode, Keyboard, ScancodeSet1};

pub async fn handle_keypresses(
    press_handler: impl Fn(DecodedKey),
) {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                press_handler(key)
            }
        }
    }
}
