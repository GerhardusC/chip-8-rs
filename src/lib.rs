use std::{error::Error, fmt::Display};

mod emulator;
mod utils;

pub use emulator::*;
pub use utils::*;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

pub trait Draw {
    fn draw_buffer(&mut self, screen_buf: &[u8]);
    fn clear_screen(&mut self);
}

pub trait ReadInputState {
    fn read_keys_state(&self) -> Result<[u8; 16], String>;
    fn reset_keys_state(&mut self);
}

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
