use std::error::Error;

use chip_eight::{Draw, Emulator, ReadInputState};

struct DebugDrawer;
impl Draw for DebugDrawer {
    fn draw_buffer(&mut self, screen_buf: &[u8]) {
        let x = screen_buf
            .chunks(64)
            .map(|s| {
                s.iter()
                    .map(|c| if *c == 0 { "  " } else { "██" })
                    .collect()
            })
            .collect::<Vec<String>>()
            .join("\n");
        println!("{}", x);
    }

    fn clear_screen(&mut self) {}
}

struct DummyInput;

impl ReadInputState for DummyInput {
    fn read_keys_state(&self) -> Result<[u8; 16], String> {
        Ok([0; 16])
    }

    fn reset_keys_state(&mut self) {}
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
cargo run --example debug_example /path/to/chip8/program.c8
"
        );
        std::process::exit(1);
    };

    let Ok(program) = std::fs::read(program_name) else {
        eprintln!("Chip 8 program not found");
        std::process::exit(2);
    };

    Emulator::init(program, DebugDrawer, DummyInput)?.debug();

    Ok(())
}
