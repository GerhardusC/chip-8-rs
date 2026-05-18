use std::{error::Error, fmt::Display, sync::atomic::AtomicU16};

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

struct Emulator {
    memory: [u8; 0x1000],
    stack: Vec<usize>,
    variable_registers: [u8; 16],
    screen_buffer: [u8; 8 * 32],
    font_addr: usize,
    index_register: usize,
    program_counter: usize,
    delay_timer: AtomicU16,
    sound_timer: AtomicU16,
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
    Unimplemented(u16),
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

impl Emulator {
    fn init() -> Result<Self, ApplicationError> {
        let mut emulator = Self {
            memory: [0; 0x1000],
            stack: vec![],
            variable_registers: [0; 16],
            screen_buffer: [0; 8 * 32],
            font_addr: 0x50,
            index_register: 0,
            program_counter: 0x200,
            delay_timer: AtomicU16::new(0),
            sound_timer: AtomicU16::new(0),
        };

        emulator.set_font(&FONT)?;

        Ok(emulator)
    }

    fn set_font(&mut self, font: &[u8]) -> Result<(), ApplicationError> {
        Ok(self.set_mem_block(font, self.font_addr)?)
    }

    fn fetch(&mut self) -> Instruction {
        let tophalf = self.memory[self.program_counter];
        let bothalf = self.memory[self.program_counter + 1];
        self.program_counter += 2;
        (((tophalf as u16) << 8) | bothalf as u16).into()
    }

    fn execute(&mut self, instruction: Instruction) {
        match instruction {
            Instruction::ClearScreen => {}
            Instruction::Draw {
                x_register,
                y_register,
                height,
            } => {}
            Instruction::Jump(address) => self.program_counter = address as usize,
            Instruction::SetIndexRegister(address) => self.index_register = address as usize,
            Instruction::SetGeneralRegister { register, value } => {}
            Instruction::AddToRegister { register, value } => {}
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

    fn run(mut self) {
        let delay_timer = &self.delay_timer;
        let sound_timer = &self.sound_timer;
        std::thread::scope(|s| {});
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut emulator = Emulator::init()?;
    let instruction = emulator.fetch();

    Ok(())
}

#[derive(Debug)]
enum ApplicationError {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_fetch_an_instruction() {
        let mut emulator = Emulator::init().expect("All initial memory is in range");

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
