use std::{
    error::Error,
    sync::{
        Arc, Mutex,
        mpsc::{self, Sender},
    },
};

use chip_eight::{Draw, Emulator, ReadInputState, SCREEN_WIDTH};
use minifb::{Window, WindowOptions};

// The emulator needs a handle to drawing to the screen, and one for reading user input. For better
// or worse, I have decided to implement them as traits that need to be implemented for types
// passed to the emulator.
//
// In this implementation is not particularly great or fancy, but it illustrates how the emulator
// will call the requird drawing functions and respond to them.
impl Draw for &App {
    fn draw_buffer(&mut self, screen_buf: &[u8]) {
        let _ = self.sender.send(screen_buf.to_vec());
    }

    fn clear_screen(&mut self) {
        let _ = self.sender.send(vec![0; 640 * 320]);
    }
}

impl ReadInputState for &App {
    fn read_keys_state(&self) -> Result<[u8; 16], String> {
        if let Ok(keys) = self.keys.lock() {
            Ok(*keys)
        } else {
            Err("Mutext lock error".to_owned())
        }
    }

    fn reset_keys_state(&mut self) {
        let new_keys = [0; 16];
        if let Ok(mut keys) = self.keys.lock() {
            *keys = new_keys;
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let keys = Arc::new(Mutex::new([0; 16]));

    let app = App {
        sender: tx.clone(),
        keys: keys.clone(),
    };

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

    std::thread::spawn(move || {
        let mut window = Window::new("Minifb Chip 8 Example", 640, 320, WindowOptions::default())
            .expect("Failed to create window");

        while window.is_open() {
            if let Ok(msg) = rx.try_recv() {
                let buffer = expand_buffer(&msg, 10, SCREEN_WIDTH);
                let _ = window.update_with_buffer(buffer.as_slice(), 640, 320);
            }
            let new_keys = &mut [0; 16];

            // b'x', b'1', b'2', b'3', b'q', b'w', b'e', b'a', b's', b'd', b'z', b'c', b'4', b'r', b'f', b'v',
            for key in window.get_keys() {
                match key {
                    minifb::Key::X => new_keys[0] = 1,
                    minifb::Key::Key1 => new_keys[1] = 1,
                    minifb::Key::Key2 => new_keys[2] = 1,
                    minifb::Key::Key3 => new_keys[3] = 1,
                    minifb::Key::Q => new_keys[4] = 1,
                    minifb::Key::W => new_keys[5] = 1,
                    minifb::Key::E => new_keys[6] = 1,
                    minifb::Key::A => new_keys[7] = 1,
                    minifb::Key::S => new_keys[8] = 1,
                    minifb::Key::D => new_keys[9] = 1,
                    minifb::Key::Z => new_keys[10] = 1,
                    minifb::Key::C => new_keys[11] = 1,
                    minifb::Key::Key4 => new_keys[12] = 1,
                    minifb::Key::R => new_keys[13] = 1,
                    minifb::Key::F => new_keys[14] = 1,
                    minifb::Key::V => new_keys[15] = 1,
                    _ => {}
                }
            }
            if let Ok(mut keys) = keys.lock() {
                *keys = *new_keys;
            }
        }
    });

    let emulator = Emulator::init(program, chip_eight::RunningMode::Normal, &app, &app)?;
    emulator.run();

    Ok(())
}

struct App {
    sender: Sender<Vec<u8>>,
    keys: Arc<Mutex<[u8; 16]>>,
}

// Really, not the most efficient function, but whatever, I just want to grow pixels by a factor
// and convert 1's to a pixel color and 0's to no pixel color (illustrated by th unit test
// 'it_can_expand_buffer'
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
