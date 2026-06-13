//! This crate provides the internal implementations for a chip 8 interpreter/emulator.
//!
//! It currently provides two external traits that need to be implemented by the user of the crate
//! (a third trait might be added if sound is ever supported), the Draw and ReadInputState traits.
//!
//! The main idea that I was hoping to achieve here was to not have any external dependencies so I
//! can load the interpreter on to almost any device I want in future without needing to rewrite the
//! entire application.
//!
//! Once the abovementioned traits are implemented, the interpreter will call these functions
//! when appropriate.
//!
//! For example implementations see the examples directory.
//!
//! # Usage:
//! ```
//! // These would be your own implementation
//! struct Drawer;
//! struct InputReader;
//!
//! impl chip_eight::Draw for Drawer {
//!     fn draw_buffer(&mut self, screen_buf: &[u8], screen_width: usize, screen_height: usize) { todo!() }
//!     fn clear_screen(&mut self) { todo!() }
//! }
//! impl chip_eight::ReadInputState for InputReader {
//!     fn read_keys_state(&self) -> Result<[u8; 16], String> { todo!() }
//!     fn reset_keys_state(&mut self) { todo!() }
//! }
//!
//! let (drawer, input_reader) = (Drawer, InputReader);
//! let program = vec![]; // Read it from somewhere
//!
//! let mut emulator = chip_eight::Emulator::init(program, drawer, input_reader)
//!     .expect("The chip 8 program is probably too large");
//!
//! emulator
//!     .set_max_draw_delay(std::time::Duration::from_millis(6))
//!     .set_quirks_mode(chip_eight::QuirksMode::Chip8);
//!
//! // Most simply you can then run the emulator as follows:
//! // EITHER: emulator.run_blocking();
//! // OR:
//! for emulator_state in emulator {
//!     // Here emulator_state gives you access to the previous instruction as well as
//!     // This is the expensive way of using the emulator, but is nice for debuggers
//!     //(Just breaking here for the doctest)
//!     break;
//! }
//! ```
use std::{error::Error, fmt::Display};

mod emulator;
mod twister_rand;
mod utils;

pub use emulator::*;

const BASE_SCREEN_WIDTH: usize = 64;
const BASE_SCREEN_HEIGHT: usize = 32;

/// Implement this trait on a type and use it to draw to or clear the screen.
pub trait Draw {
    /// This function will be called each time the interpreter requests to draw to the screen. The
    /// buffer you are given is just a flat list, but you are given the width and height so you can
    /// make it square yourself.
    fn draw_buffer(&mut self, screen_buf: &[u8], screen_width: usize, screen_height: usize);

    /// Clear screen will simply be called when the interpreter calls the clear screen instruction.
    fn clear_screen(&mut self);
}

/// Implement this trait on a type and use it to read input state.
pub trait ReadInputState {
    /// This function is called whenever the interpreter needs to read keyboard state. You need to
    /// give back an array of 16 characters, each mapping to the 0x0 - 0xF keys on the hex keyboard
    fn read_keys_state(&self) -> Result<[u8; 16], String>;

    /// This is a method that most of the time doesn't need to be implemented, you can just perform
    /// a no-op, but it does give you access to when the emulator is no longer reading a key. This
    /// helps particularly only when you are in an environment where you don't have continuous
    /// access to the keys state and you have to keep track of it yourself. I mean, I ran into this
    /// in the examples directory under my silly implementation for the normal terminal window, but
    /// it could just be a skill issue.
    fn reset_keys_state(&mut self);
}

/// Currently the only ApplicationError variant is the program being too large to fit in the
/// interpreter's memory.
#[derive(Debug)]
pub enum ApplicationError {
    MemoryLocationOutOfRange { max_addr: usize },
}

impl Error for ApplicationError {}

impl Display for ApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplicationError::MemoryLocationOutOfRange { max_addr } => {
                write!(
                    f,
                    "Tried to write memory out of range, max address: {max_addr}"
                )
            }
        }
    }
}
