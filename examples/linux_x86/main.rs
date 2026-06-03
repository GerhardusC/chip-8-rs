mod debug_draw;
mod dummy_input;
mod silly_draw;
mod silly_input;

use std::{error::Error, time::Duration};

use chip_eight::{Emulator, ReadInputState, RunningMode};

use crate::{
    debug_draw::DebugDrawer, dummy_input::DummyInput, silly_draw::Drawer,
    silly_input::InputListener,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args();
    let num_args = args.len();

    let mut running_mode = RunningMode::Normal;
    let mut program_name = None;

    for (i, arg) in args.enumerate() {
        if arg == "--test-input" || arg == "-t" {
            let input_state = InputListener::init();
            loop {
                let keys = input_state.read_keys_state()?;
                if let Some(key) = keys.last()
                    && *key > 0
                {
                    eprintln!("EXITING");
                    return Ok(());
                }
                dbg!(keys);
                std::thread::sleep(Duration::from_millis(6));
            }
        } else if arg == "--dbg" || arg == "-d" {
            running_mode = RunningMode::Debug;
        } else if arg == "--help" || arg == "-h" {
            eprintln!(
                r#"Chip 8
USAGE: [program_name] [COMMANDS...] <program_name>
    --dbg | -d: Debug mode

Example:
    chip-eight-rs --dbg test_program.c8"#
            );
            return Ok(());
        }
        if i == num_args - 1 && i > 0 {
            let _ = program_name.insert(arg.to_owned());
        }
    }

    let Some(input) = program_name else {
        eprintln!("Program is a required final argument");
        std::process::exit(1);
    };

    let Ok(input) = std::fs::read(input) else {
        eprintln!("Test program does not exist");
        std::process::exit(2);
    };

    match running_mode {
        RunningMode::Debug => {
            let emulator: Emulator<DebugDrawer, DummyInput> =
                Emulator::init(input, running_mode, DebugDrawer, DummyInput)?;
            emulator.run();
        }
        RunningMode::Normal => {
            let emulator: Emulator<Drawer, InputListener> =
                Emulator::init(input, running_mode, Drawer::init(), InputListener::init())?;
            emulator.run();
        }
    }

    Ok(())
}
