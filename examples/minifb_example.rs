use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
        mpsc::{self, Sender},
    },
};

use chip_eight::{Draw, Emulator, ReadInputState};

// The emulator needs a handle to drawing to the screen, and one for reading user input. For better
// or worse, I have decided to implement them as traits that need to be implemented for types
// passed to the emulator.
//
// In this implementation is not particularly great or fancy, but it illustrates how the emulator
// will call the requird drawing functions and respond to them.
impl Draw for DisplayOutput {
    fn draw_buffer(&mut self, screen_buf: &[u8]) {}

    fn clear_screen(&mut self) {}
}

impl ReadInputState for KeyboardInput {
    fn read_keys_state(&self) -> Result<[u8; 16], String> {
        let x = std::array::from_fn(|i| self.keys[i].load(Ordering::Relaxed));
        Ok(x)
    }

    fn reset_keys_state(&mut self) {
        for key in self.keys.iter() {
            key.store(0, Ordering::Relaxed);
        }
    }
}

struct KeyboardInput {
    keys: Arc<[AtomicU8; 16]>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args();

    let program_name = if args.len() > 1
        && let Some(arg) = args.last()
    {
        arg
    } else {
        eprintln!(
            "Chip 8 Program needs to be passed as final argument

[USAGE]
cargo run --example minifb_example /path/to/chip8/program.c8
"
        );
        std::process::exit(1);
    };

    let Ok(program) = std::fs::read(program_name) else {
        eprintln!("Chip 8 program not found");
        std::process::exit(2);
    };

    let display_output = DisplayOutput;

    let keyboard_input = KeyboardInput {
        keys: Arc::new(std::array::from_fn(|_| AtomicU8::new(0))),
    };

    Emulator::init(
        program,
        chip_eight::RunningMode::Normal,
        display_output,
        keyboard_input,
    )?
    .run();

    Ok(())
}

struct DisplayOutput;

// Really, not the most efficient function, but whatever, I just want to grow pixels by a factor
// and convert 1's to a pixel color and 0's to no pixel color (illustrated by th unit test
// 'it_can_expand_buffer'
#[allow(unused)]
fn expand_buffer(buf: &[u8], factor: u8, screen_width: usize) -> Vec<u32> {
    let mut output = vec![];

    for chunk in buf.chunks(screen_width) {
        let mut acc = vec![];
        for c in chunk {
            for _ in 0..factor {
                acc.push(if *c > 0 { 0xFFFFFF00 } else { 0x0 });
            }
        }

        for _ in 0..factor {
            output.extend(acc.as_slice());
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_expand_buffer() {
        #[rustfmt::skip]
        let input = [
            1, 0,
            0, 1
        ];
        #[rustfmt::skip]
        let expected = vec![
            0xFFFFFF00, 0xFFFFFF00, 0x00000000, 0x00000000,
            0xFFFFFF00, 0xFFFFFF00, 0x00000000, 0x00000000,
            0x00000000, 0x00000000, 0xFFFFFF00, 0xFFFFFF00,
            0x00000000, 0x00000000, 0xFFFFFF00, 0xFFFFFF00,
        ];

        let output = expand_buffer(&input, 2, 2);

        assert_eq!(output, expected);
    }
}
