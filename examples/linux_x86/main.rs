mod silly_draw;
mod silly_input;

use std::error::Error;

use chip_eight::{Emulator, ReadInputState, RunningMode};

use crate::{silly_draw::Drawer, silly_input::InputListener};

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
cargo run --example linux_x86 /path/to/chip8/program.c8
"
        );
        std::process::exit(1);
    };

    let Ok(program) = std::fs::read(program_name) else {
        eprintln!("Chip 8 program not found");
        std::process::exit(2);
    };

    Emulator::init(
        program,
        RunningMode::Normal,
        Drawer::init(),
        InputListener::init(),
    )?
    .run();

    Ok(())
}
