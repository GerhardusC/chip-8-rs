use std::{error::Error, fmt::Display};

mod debug_draw;
mod silly_draw;

pub use debug_draw::*;
pub use silly_draw::*;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

pub trait Draw {
    fn init() -> Self;
    fn draw_buffer(&self, screen_buf: &[u8]);
    fn return_home(&self);
    fn clear_screen(&self);
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
