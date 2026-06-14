use std::error::Error;

use chip_eight::{Draw, Emulator, ReadInputState};

struct DebugDrawer;
impl Draw for DebugDrawer {
    fn draw_buffer(&mut self, screen_buf: &[u8], screen_width: usize, _screen_height: usize) {
        let x = screen_buf
            .chunks(screen_width)
            .map(|s| s.iter().map(|c| if *c == 0 { ' ' } else { '█' }).collect())
            .collect::<Vec<String>>()
            .join("\n");
        println!("{}", x);
    }
}

struct DummyInput;

impl ReadInputState for DummyInput {
    fn read_keys_state(&self) -> Result<[u8; 16], String> {
        println!("\x1b[32;1mENTER ALL PRESSED KEYS:\x1b[0m");
        let mut keys = [
            b'x', b'1', b'2', b'3', b'q', b'w', b'e', b'a', b's', b'd', b'z', b'c', b'4', b'r',
            b'f', b'v',
        ];

        let mut res = String::new();
        let Ok(_) = std::io::stdin().read_line(&mut res) else {
            return Ok([0; 16]);
        };
        for key in keys.iter_mut() {
            if res.contains(char::from(*key)) {
                *key = 1;
            } else {
                *key = 0;
            }
        }

        Ok(keys)
    }
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

    let mut emulator = Emulator::init(program.clone(), DebugDrawer, DummyInput)?;

    let mut prev_index_register = 0;
    let mut prev_program_counter = 0;
    let mut prev_stack = String::new();
    let mut prev_memory = String::new();
    let mut prev_varaible_registers = String::new();
    let mut prev_screen_buffer = String::new();

    loop {
        let emulator_state = emulator
            .next()
            .expect("There will always be a next instruction");

        dbg!(&emulator_state.last_instruction);

        let index_register = emulator_state.index_register;
        if prev_index_register != index_register {
            dbg!(&index_register);
            prev_index_register = index_register;
        }
        let program_counter = emulator_state.program_counter;
        if prev_program_counter != program_counter {
            dbg!(&program_counter);
            prev_program_counter = program_counter;
        }
        let stack = format!("{:?}", &emulator_state.stack);
        if prev_stack != stack {
            dbg!(&stack);
            prev_stack = stack;
        }
        let memory = format!("{:?}", emulator_state.memory);
        if prev_memory != memory {
            println!("Memory updated");
            prev_memory = memory;
        }
        let varaible_registers = format!("{:?}", emulator_state.variable_registers);
        if prev_varaible_registers != varaible_registers {
            dbg!(&varaible_registers);
            prev_varaible_registers = varaible_registers;
        }
        let screen_buffer = format!("{:?}", emulator_state.screen_buffer);
        if prev_screen_buffer != screen_buffer {
            dbg!("Screen_updated");
            prev_screen_buffer = screen_buffer;
        }

        println!("Next instruction: 'n'");
        let mut res = String::new();
        let Ok(_) = std::io::stdin().read_line(&mut res) else {
            break;
        };
        match res.trim() {
            "r" => {
                emulator.reset(program.clone()).expect("Program too large");
            }
            "q" => {
                break;
            }
            _ => {
                continue;
            }
        }
        if res.trim() != "q" {
            continue;
        } else {
            break;
        }
    }

    Ok(())
}
