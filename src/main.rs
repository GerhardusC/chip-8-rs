use std::{error::Error, sync::atomic::AtomicU16, time::Duration};

use chip_eight::{ApplicationError, DebugDrawer, Draw, Drawer, SCREEN_HEIGHT, SCREEN_WIDTH};

const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

struct Emulator<T: Draw> {
    memory: [u8; 0x1000],
    #[allow(unused)]
    stack: Vec<usize>,
    variable_registers: [u8; 16],
    screen_buffer: [u8; SCREEN_WIDTH * SCREEN_HEIGHT],
    font_addr: usize,
    index_register: usize,
    program_counter: usize,
    #[allow(unused)]
    delay_timer: AtomicU16,
    #[allow(unused)]
    sound_timer: AtomicU16,
    drawer: T,
}

#[derive(Debug)]
enum Instruction {
    // 00E0 (clear screen)
    ClearScreen,
    // 1NNN (jump)
    Jump(u16),
    // 6XNN (set register VX)
    SetIndexRegister(u16),
    // 7XNN (add value to register VX)
    SetGeneralRegister {
        register: usize,
        value: u8,
    },
    // ANNN (set index register I)
    AddToRegister {
        register: usize,
        value: u8,
    },
    // DXYN (display/draw)
    Draw {
        x_register: usize,
        y_register: usize,
        height: u8,
    },
    #[allow(unused)]
    Unimplemented(u16),
    #[allow(unused)]
    Error(u16),
}

impl From<u16> for Instruction {
    fn from(value: u16) -> Self {
        match value >> 12 {
            // 00E0 (clear screen)
            0x0 => Self::ClearScreen,
            // 1NNN (jump)
            0x1 => Self::Jump(0x0FFF & value),
            0x2 => Self::Unimplemented(value),
            0x3 => Self::Unimplemented(value),
            0x4 => Self::Unimplemented(value),
            0x5 => Self::Unimplemented(value),
            // 6XNN (set register VX)
            0x6 => Self::SetGeneralRegister {
                register: (0xF00 & value) as usize >> 8,
                value: (0xFF & value) as u8,
            },
            // 7XNN (add value to register VX)
            0x7 => Self::AddToRegister {
                register: (0xF00 & value) as usize >> 8,
                value: (0xFF & value) as u8,
            },
            0x8 => Self::Unimplemented(value),
            0x9 => Self::Unimplemented(value),
            // ANNN (set index register I)
            0xA => Self::SetIndexRegister(0xFFF & value),
            0xB => Self::Unimplemented(value),
            0xC => Self::Unimplemented(value),
            // DXYN (display/draw)
            0xD => Self::Draw {
                x_register: (0xF00 & value) as usize >> 8,
                y_register: (0xF0 & value) as usize >> 4,
                height: (0xF & value) as u8,
            },
            0xE => Self::Unimplemented(value),
            0xF => Self::Unimplemented(value),

            _ => unreachable!(
                "By bitshifting the value 12 to the right, we only have 4 bits, i.e. 0x0-0xF as insturctions"
            ),
        }
    }
}

impl<T: Draw> Emulator<T> {
    // TODO: take in the program memory as argument and copy it into memory in the constructor.
    fn init() -> Result<Self, ApplicationError> {
        let mut emulator = Self {
            memory: [0; 0x1000],
            stack: vec![],
            variable_registers: [0; 0x10],
            screen_buffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT],
            font_addr: 0x50,
            index_register: 0,
            program_counter: 0x200,
            delay_timer: AtomicU16::new(0),
            sound_timer: AtomicU16::new(0),
            drawer: T::init(),
        };

        emulator.set_font(&FONT)?;

        Ok(emulator)
    }

    fn set_font(&mut self, font: &[u8]) -> Result<(), ApplicationError> {
        self.set_mem_block(font, self.font_addr)
    }

    fn fetch(&mut self) -> Instruction {
        let tophalf = self.memory[self.program_counter];
        let bothalf = self.memory[self.program_counter + 1];
        self.program_counter += 2;
        (((tophalf as u16) << 8) | bothalf as u16).into()
    }

    fn execute(&mut self, instruction: Instruction) {
        match instruction {
            Instruction::ClearScreen => {
                self.drawer.clear_screen();
            }
            Instruction::Draw {
                x_register,
                y_register,
                height,
            } => {
                self.variable_registers[0xF] = 0;
                let x_value = self.variable_registers[x_register] as u16;
                let y_value = self.variable_registers[y_register] as u16;
                let start_loc = y_value * SCREEN_WIDTH as u16 + x_value;

                // For each row in the sprite
                for i in 0..height {
                    let sprite = self.memory[self.index_register + i as usize];

                    let current_loc = start_loc + SCREEN_WIDTH as u16 * i as u16;

                    // For each pixel in the row
                    for j in 0..8 {
                        let mask: u8 = 0b10000000 >> j;
                        if let Some(x) = self.screen_buffer.get_mut(current_loc as usize + j) {
                            let ans = mask & sprite;
                            let tmp = if ans > 0 { 1 } else { 0 };
                            if tmp == 1 && *x == tmp {
                                self.variable_registers[0xF] = 1;
                            } else {
                                *x = tmp;
                            }
                        }
                    }
                }
                self.drawer.draw_buffer(&self.screen_buffer);
            }
            Instruction::Jump(address) => self.program_counter = address as usize,
            Instruction::SetIndexRegister(address) => self.index_register = address as usize,
            Instruction::SetGeneralRegister { register, value } => {
                self.variable_registers[register] = value
            }
            Instruction::AddToRegister { register, value } => {
                self.variable_registers[register] += value
            }
            Instruction::Unimplemented(_) => {}
            Instruction::Error(_) => {}
        }
    }

    fn set_mem_block(&mut self, set: &[u8], start_addr: usize) -> Result<(), ApplicationError> {
        let end_addr = start_addr + set.len();
        if end_addr > self.memory.len() {
            return Err(ApplicationError::MemoryLocationOutOfRange {
                max_addr: self.memory.len() - set.len(),
            });
        }
        let x = &mut self.memory[start_addr..end_addr];
        for (i, item) in x.iter_mut().enumerate() {
            *item = set[i];
        }
        Ok(())
    }

    fn _run(self) {
        let _delay_timer = &self.delay_timer;
        let _sound_timer = &self.sound_timer;
        std::thread::scope(|_s| {});
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args();
    args.next();
    let mode = args.next();
    let input = std::fs::read("./test_logo_program").expect("test program should exist");

    match mode.as_ref() {
        Some(x) if x == "--dbg" || x == "-d" => {
            let mut emulator = Emulator::<DebugDrawer>::init()?;
            emulator.set_mem_block(&input, 0x200)?;
            loop {
                let instruction = emulator.fetch();
                emulator.execute(instruction);
                println!("Next instruction: 'n'");
                let mut res = String::new();
                std::io::stdin().read_line(&mut res)?;
                if res.trim() == "n" {
                    continue;
                } else {
                    return Ok(());
                }
            }
        }
        _ => {
            let mut emulator = Emulator::<Drawer>::init()?;
            emulator.set_mem_block(&input, 0x200)?;
            loop {
                let instruction = emulator.fetch();
                emulator.execute(instruction);
                std::thread::sleep(Duration::from_millis(100 / 6));
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_fetch_an_instruction() {
        let mut emulator = Emulator::<Drawer>::init().expect("All initial memory is in range");

        emulator
            .set_font(&FONT)
            .expect("Should be able to set font");

        // Clear
        emulator.memory[0x200] = 0x00;
        emulator.memory[0x201] = 0xE0;

        // Draw XYH
        emulator.memory[0x202] = 0xDE;
        emulator.memory[0x203] = 0xF5;

        // 6XNN (set register VX)
        emulator.memory[0x204] = 0x6E;
        emulator.memory[0x205] = 0xAB;

        // 7XNN (add value to register VX)
        emulator.memory[0x206] = 0x7E;
        emulator.memory[0x207] = 0xAB;

        // ANNN (set index register I)
        emulator.memory[0x208] = 0xA1;
        emulator.memory[0x209] = 0x23;

        // 1NNN (jump)
        emulator.memory[0x20A] = 0x11;
        emulator.memory[0x20B] = 0x23;

        emulator.program_counter = 0x200;

        let instruction = emulator.fetch();
        let instruction2 = emulator.fetch();
        let instruction3 = emulator.fetch();
        let instruction4 = emulator.fetch();
        let instruction5 = emulator.fetch();
        let instruction6 = emulator.fetch();
        assert!(matches!(instruction, Instruction::ClearScreen));
        assert!(matches!(
            instruction2,
            Instruction::Draw {
                x_register: 0xE,
                y_register: 0xF,
                height: 0x5
            }
        ));
        assert!(matches!(
            instruction3,
            Instruction::SetGeneralRegister {
                register: 0xE,
                value: 0xAB
            }
        ));
        assert!(matches!(
            instruction4,
            Instruction::AddToRegister {
                register: 0xE,
                value: 0xAB
            }
        ));
        assert!(matches!(instruction5, Instruction::SetIndexRegister(0x123)));
        assert!(matches!(instruction6, Instruction::Jump(0x123)));
    }
}
