use std::{
    sync::{
        Arc,
        atomic::{AtomicU16, Ordering},
    },
    time::{Duration, UNIX_EPOCH},
};

use crate::{ApplicationError, Draw, ReadInputState, SCREEN_HEIGHT, SCREEN_WIDTH, u8_to_arr};

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

pub struct Emulator<T: Draw, P: ReadInputState> {
    memory: [u8; 0x1000],
    // TODO: Check if stack pointer needs to exist
    #[allow(unused)]
    stack: Vec<usize>,
    #[allow(unused)]
    keys: [u8; 16],
    variable_registers: [u8; 16],
    screen_buffer: [u8; SCREEN_WIDTH * SCREEN_HEIGHT],
    font_addr: usize,
    index_register: usize,
    program_counter: usize,
    #[allow(unused)]
    delay_timer: Arc<AtomicU16>,
    #[allow(unused)]
    sound_timer: Arc<AtomicU16>,
    drawer: T,
    #[allow(unused)]
    input_provider: P,
    running_mode: RunningMode,
}

#[derive(Debug)]
enum LogicalOperator {
    // 8XY0: Set
    // VX is set to the value of VY.
    Set,
    // 8XY1: Binary OR
    // VX is set to the bitwise/binary logical disjunction (OR) of VX and VY. VY is not affected.
    BinaryOr,
    // 8XY2: Binary AND
    // VX is set to the bitwise/binary logical conjunction (AND) of VX and VY. VY is not affected.
    BinaryAnd,
    // 8XY3: Logical XOR
    // VX is set to the bitwise/binary exclusive OR (XOR) of VX and VY. VY is not affected.
    LogicalXor,
    // 8XY4: Add
    // VX is set to the value of VX plus the value of VY. VY is not affected.
    // Unlike 7XNN, this addition will affect the carry flag. If the result is larger than 255 (and thus overflows the 8-bit register VX), the flag register VF is set to 1. If it doesn’t overflow, VF is set to 0.
    AddAffectingCarry,
    // 8XY5 and 8XY7: Subtract
    // These both subtract the value in one register from the other, and put the result in VX. In both cases, VY is not affected.
    // 8XY5 sets VX to the result of VX - VY.
    // This subtraction will also affect the carry flag, but note that it’s opposite from what you might think. If the minuend (the first operand) is larger than or equal to the subtrahend (second operand), VF will be set to 1. If the subtrahend is larger, and we “underflow” the result, VF is set to 0. Another way of thinking of it is that VF is set to 1 before the subtraction, and then the subtraction either borrows from VF (setting it to 0) or not.
    Subtract,
    // 8XY7 sets VX to the result of VY - VX.
    SubtractReverse,
    Shift(Direction),
    Invalid,
}

#[derive(Debug)]
enum Direction {
    Left,
    Right,
}

impl From<u16> for LogicalOperator {
    fn from(value: u16) -> Self {
        let relevant_byte = value & 0xF;
        match relevant_byte {
            0x0 => Self::Set,
            0x1 => Self::BinaryOr,
            0x2 => Self::BinaryAnd,
            0x3 => Self::LogicalXor,
            0x4 => Self::AddAffectingCarry,
            0x5 => Self::Subtract,
            0x6 | 0xE => Self::Shift(if relevant_byte == 0x6 {
                Direction::Right
            } else {
                Direction::Left
            }),
            0x7 => Self::SubtractReverse,
            _ => Self::Invalid,
        }
    }
}

#[derive(Debug)]
enum Instruction {
    // 00E0 (clear screen)
    ClearScreen,
    // 00EE (return)
    Return,
    // 2NNN (Subroutine)
    Call(usize),
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
    SkipEqValueWithRegisterContents {
        register: usize,
        value: u8,
    },
    SkipNotEqValueWithRegisterContents {
        register: usize,
        value: u8,
    },
    SkipEqRegisters {
        register_x: usize,
        register_y: usize,
    },
    SkipNotEqRegisters {
        register_x: usize,
        register_y: usize,
    },
    LogicalOperator {
        operator: LogicalOperator,
        register_x: usize,
        register_y: usize,
    },
    JumpWithOffset {
        register_x: usize,
        address: usize,
    },
    Random {
        register_x: usize,
        val_to_and: u8,
    },
    FCommand {
        register: usize,
        command: FCommand,
    },
    #[allow(unused)]
    Unimplemented(u16),
    #[allow(unused)]
    Error(u16),
}

#[derive(Debug)]
enum FCommand {
    // Timers
    ReadDelayTimer,
    SetDelayTimer,
    SetSoundTimer,

    // Aux
    AddToIndexRegister,
    GetFontCharacter,
    DecimalConversion,

    // Memory
    StoreTo,
    LoadFrom,

    // Input
    GetKey,

    Unimplemented(u16),
}

impl From<u16> for FCommand {
    fn from(value: u16) -> Self {
        match value & 0xFF {
            // Timers
            0x07 => Self::ReadDelayTimer,
            0x15 => Self::SetDelayTimer,
            0x16 => Self::SetSoundTimer,

            // Aux
            0x1E => Self::AddToIndexRegister,
            0x29 => Self::GetFontCharacter,
            0x33 => Self::DecimalConversion,

            // Memory
            0x55 => Self::StoreTo,
            0x65 => Self::LoadFrom,

            // Input
            0x0A => Self::GetKey,

            _ => Self::Unimplemented(value),
        }
    }
}

impl From<u16> for Instruction {
    fn from(value: u16) -> Self {
        match value >> 12 {
            // 00E0 (clear screen)
            0x0 => {
                if value == 0x00EE {
                    Self::Return
                } else if value == 0x00E0 {
                    Self::ClearScreen
                } else {
                    Self::Unimplemented(value)
                }
            }
            // 1NNN (jump)
            0x1 => Self::Jump(0x0FFF & value),
            0x2 => Self::Call((0x0FFF & value) as usize),
            0x3 => Self::SkipEqValueWithRegisterContents {
                register: (value & 0x0F00) as usize >> 8,
                value: (value & 0x00FF) as u8,
            },
            0x4 => Self::SkipNotEqValueWithRegisterContents {
                register: (value & 0x0F00) as usize >> 8,
                value: (value & 0x00FF) as u8,
            },
            0x5 => Self::SkipEqRegisters {
                register_x: (value & 0x0F00) as usize >> 8,
                register_y: (value & 0x00F0) as usize >> 4,
            },
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
            0x8 => Self::LogicalOperator {
                operator: LogicalOperator::from(value),
                register_x: (0x0F00 & value) as usize >> 8,
                register_y: (0x00F0 & value) as usize >> 4,
            },
            0x9 => Self::SkipNotEqRegisters {
                register_x: (value & 0x0F00) as usize >> 8,
                register_y: (value & 0x00F0) as usize >> 4,
            },
            // ANNN (set index register I)
            0xA => Self::SetIndexRegister(0xFFF & value),
            // BNNN: Jump with offset
            0xB => Self::JumpWithOffset {
                register_x: (value & 0x0F00) as usize >> 8,
                address: (value & 0x0FFF) as usize,
            },
            0xC => Self::Random {
                register_x: (value & 0x0F00) as usize >> 8,
                val_to_and: (value & 0x00FF) as u8,
            },
            // DXYN (display/draw)
            0xD => Self::Draw {
                x_register: (0xF00 & value) as usize >> 8,
                y_register: (0xF0 & value) as usize >> 4,
                height: (0xF & value) as u8,
            },
            0xE => Self::Unimplemented(value),
            0xF => Self::FCommand {
                register: (0xF00 & value) as usize >> 8,
                command: FCommand::from(value),
            },

            _ => unreachable!(
                "By bitshifting the value 12 to the right, we only have 4 bits, i.e. 0x0-0xF as insturctions"
            ),
        }
    }
}

impl<T: Draw, P: ReadInputState> Emulator<T, P> {
    pub fn init(program: Vec<u8>, running_mode: RunningMode) -> Result<Self, ApplicationError> {
        let mut emulator = Self {
            memory: [0; 0x1000],
            stack: vec![],
            keys: [0; 16],
            variable_registers: [0; 16],
            screen_buffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT],
            font_addr: 0x50,
            index_register: 0,
            program_counter: 0x200,
            delay_timer: Arc::new(AtomicU16::new(0)),
            sound_timer: Arc::new(AtomicU16::new(0)),
            drawer: T::init(),
            running_mode,
            input_provider: P::init(),
        };

        emulator.set_mem_block(&program, 0x200)?;
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
                let val: u16 = value as u16 + self.variable_registers[register] as u16;
                self.variable_registers[register] = (val & 0xFF) as u8;
            }
            Instruction::SkipEqValueWithRegisterContents { register, value } => {
                let vx_value = self.variable_registers[register];
                if vx_value == value {
                    self.program_counter += 2;
                }
            }
            Instruction::SkipNotEqValueWithRegisterContents { register, value } => {
                let vx_value = self.variable_registers[register];
                if vx_value != value {
                    self.program_counter += 2;
                }
            }
            Instruction::SkipEqRegisters {
                register_x,
                register_y,
            } => {
                let vx_value = self.variable_registers[register_x];
                let vy_value = self.variable_registers[register_y];
                if vx_value == vy_value {
                    self.program_counter += 2;
                }
            }
            Instruction::SkipNotEqRegisters {
                register_x,
                register_y,
            } => {
                let vx_value = self.variable_registers[register_x];
                let vy_value = self.variable_registers[register_y];
                if vx_value != vy_value {
                    self.program_counter += 2;
                }
            }
            Instruction::LogicalOperator {
                operator,
                register_x,
                register_y,
            } => {
                match operator {
                    LogicalOperator::Set => {
                        self.variable_registers[register_x] = self.variable_registers[register_y];
                    }
                    LogicalOperator::BinaryOr => {
                        self.variable_registers[register_x] |= self.variable_registers[register_y];
                    }
                    LogicalOperator::BinaryAnd => {
                        self.variable_registers[register_x] &= self.variable_registers[register_y];
                    }
                    LogicalOperator::LogicalXor => {
                        self.variable_registers[register_x] ^= self.variable_registers[register_y];
                    }
                    LogicalOperator::AddAffectingCarry => {
                        let res = self.variable_registers[register_x] as u16
                            + self.variable_registers[register_y] as u16;
                        self.variable_registers[register_x] = (res & 0xFF) as u8;
                        self.variable_registers[0xF] = if res > 255 { 1 } else { 0 };
                    }
                    LogicalOperator::Subtract => {
                        let res = self.variable_registers[register_x] as i16
                            - self.variable_registers[register_y] as i16;
                        self.variable_registers[register_x] = (res & 0xFF) as u8;
                        self.variable_registers[0xF] = if res >= 0 { 1 } else { 0 };
                    }
                    LogicalOperator::SubtractReverse => {
                        let res = self.variable_registers[register_y] as i16
                            - self.variable_registers[register_x] as i16;
                        self.variable_registers[register_x] = (res & 0xFF) as u8;
                        self.variable_registers[0xF] = if res >= 0 { 1 } else { 0 };
                    }
                    LogicalOperator::Shift(direction) => {
                        // TODO: Possible feature to optionally set VX to the value of VY
                        match direction {
                            Direction::Left => {
                                // NEXT LINE ONLY IN QUIRKS
                                self.variable_registers[register_x] =
                                    self.variable_registers[register_y];

                                let top = self.variable_registers[register_y] & 0b1000_0000;
                                let res = self.variable_registers[register_y] << 1;
                                self.variable_registers[register_x] = res;
                                if top > 0 {
                                    self.variable_registers[0xF] = 1
                                } else {
                                    self.variable_registers[0xF] = 0
                                };
                            }
                            Direction::Right => {
                                // NEXT LINE ONLY IN QUIRKS
                                self.variable_registers[register_x] =
                                    self.variable_registers[register_y];

                                let bot = self.variable_registers[register_y] & 0b1;
                                let res = self.variable_registers[register_y] >> 1;
                                self.variable_registers[register_x] = res;
                                if bot > 0 {
                                    self.variable_registers[0xF] = 1
                                } else {
                                    self.variable_registers[0xF] = 0
                                };
                            }
                        }
                    }
                    LogicalOperator::Invalid => {}
                }
            }
            Instruction::JumpWithOffset {
                register_x,
                address,
            } => {
                self.program_counter = address + self.variable_registers[register_x] as usize;
            }
            Instruction::Random {
                register_x,
                val_to_and,
            } => {
                let randint = (std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Now is always after unix epoch")
                    .as_micros()
                    % 255) as u8;
                self.variable_registers[register_x] = randint & val_to_and;
            }
            Instruction::Return => {
                if let Some(val) = self.stack.pop() {
                    self.program_counter = val;
                };
            }
            Instruction::Call(memory_addr) => {
                self.stack.push(self.program_counter);
                self.program_counter = memory_addr;
            }
            Instruction::FCommand { register, command } => match command {
                FCommand::ReadDelayTimer => {
                    self.variable_registers[register] =
                        self.delay_timer.load(Ordering::Relaxed) as u8;
                }
                FCommand::SetDelayTimer => {
                    self.delay_timer
                        .store(self.variable_registers[register] as u16, Ordering::Relaxed);
                }
                FCommand::SetSoundTimer => {
                    self.sound_timer
                        .store(self.variable_registers[register] as u16, Ordering::Relaxed);
                }
                FCommand::AddToIndexRegister => {
                    self.index_register += self.variable_registers[register] as usize;
                }
                FCommand::GetFontCharacter => {
                    self.index_register =
                        self.font_addr + self.variable_registers[register] as usize;
                }
                FCommand::DecimalConversion => {
                    let val = self.variable_registers[register];
                    let val = u8_to_arr(val);

                    for (i, val) in val.iter().enumerate() {
                        self.memory[self.index_register + i] = *val;
                    }
                }
                FCommand::LoadFrom => {
                    for (i, reg) in self.memory
                        [self.index_register..=(self.index_register + register)]
                        .iter()
                        .enumerate()
                    {
                        self.variable_registers[i] = *reg;
                    }
                }
                FCommand::StoreTo => {
                    for (i, reg) in self.variable_registers[0..=register].iter().enumerate() {
                        self.memory[self.index_register + i] = *reg;
                    }
                }
                FCommand::GetKey => {
                    todo!();
                }
                FCommand::Unimplemented(value) => {
                    eprintln!("COMMAND {value} UNIMPLEMENTED");
                }
            },
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

    pub fn run(mut self) {
        let _delay_timer = &self.delay_timer;
        let _sound_timer = &self.sound_timer;
        std::thread::scope(|_s| {
            match self.running_mode {
                RunningMode::Debug => {
                    let mut prev_index_register = self.index_register;
                    let mut prev_program_counter = self.program_counter;
                    let mut prev_stack = format!("{:?}", self.stack);
                    let mut prev_memory = format!("{:?}", self.memory);
                    let mut prev_varaible_registers = format!("{:?}", self.variable_registers);
                    let mut prev_screen_buffer = format!("{:?}", self.screen_buffer);

                    loop {
                        let instruction = self.fetch();
                        dbg!(&instruction);
                        self.execute(instruction);

                        let index_register = self.index_register;
                        if prev_index_register != index_register {
                            dbg!(&index_register);
                            prev_index_register = index_register;
                        }
                        let program_counter = self.program_counter;
                        if prev_program_counter != program_counter {
                            dbg!(&program_counter);
                            prev_program_counter = program_counter;
                        }
                        let stack = format!("{:?}", &self.stack);
                        if prev_stack != stack {
                            dbg!(&stack);
                            prev_stack = stack;
                        }
                        let memory = format!("{:?}", self.memory);
                        if prev_memory != memory {
                            println!("Memory updated");
                            prev_memory = memory;
                        }
                        let varaible_registers = format!("{:?}", self.variable_registers);
                        if prev_varaible_registers != varaible_registers {
                            dbg!(&varaible_registers);
                            prev_varaible_registers = varaible_registers;
                        }
                        let screen_buffer = format!("{:?}", self.screen_buffer);
                        if prev_screen_buffer != screen_buffer {
                            dbg!("Screen_updated");
                            prev_screen_buffer = screen_buffer;
                        }

                        println!("Next instruction: 'n'");
                        let mut res = String::new();
                        let Ok(_) = std::io::stdin().read_line(&mut res) else {
                            break;
                        };
                        if res.trim() != "q" {
                            continue;
                        } else {
                            break;
                        }
                    }
                }
                RunningMode::Normal => loop {
                    let instruction = self.fetch();
                    self.execute(instruction);
                    std::thread::sleep(Duration::from_millis(6));
                },
            };
        });
    }
}

pub enum RunningMode {
    Debug,
    Normal,
}

#[cfg(test)]
mod tests {
    use crate::{DebugDrawer, DummyInput};

    use super::*;

    #[test]
    fn it_can_fetch_an_instruction() {
        let mut emulator = Emulator::<DebugDrawer, DummyInput>::init(vec![], RunningMode::Normal)
            .expect("All initial memory is in range");

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

        // 3XNN (skip if *reg X eq val NN)
        emulator.memory[0x20C] = 0x31;
        emulator.memory[0x20D] = 0x23;

        // 4XNN (skip if *reg X not eq val NN)
        emulator.memory[0x20E] = 0x41;
        emulator.memory[0x20F] = 0x23;

        // 5XY0 (skip if *reg X not eq *reg Y)
        emulator.memory[0x210] = 0x51;
        emulator.memory[0x211] = 0x20;

        // 9XY0 (skip if *reg X not eq *reg Y)
        emulator.memory[0x212] = 0x91;
        emulator.memory[0x213] = 0x20;

        emulator.program_counter = 0x200;

        let instruction = emulator.fetch();
        assert!(matches!(instruction, Instruction::ClearScreen));

        let instruction = emulator.fetch();
        assert!(matches!(
            instruction,
            Instruction::Draw {
                x_register: 0xE,
                y_register: 0xF,
                height: 0x5
            }
        ));

        let instruction = emulator.fetch();
        assert!(matches!(
            instruction,
            Instruction::SetGeneralRegister {
                register: 0xE,
                value: 0xAB
            }
        ));

        let instruction = emulator.fetch();
        assert!(matches!(
            instruction,
            Instruction::AddToRegister {
                register: 0xE,
                value: 0xAB
            }
        ));

        let instruction = emulator.fetch();
        assert!(matches!(instruction, Instruction::SetIndexRegister(0x123)));

        let instruction = emulator.fetch();
        assert!(matches!(instruction, Instruction::Jump(0x123)));

        let instruction = emulator.fetch();
        assert!(matches!(
            instruction,
            Instruction::SkipEqValueWithRegisterContents {
                register: 0x1,
                value: 0x23
            }
        ));

        let instruction = emulator.fetch();
        assert!(matches!(
            instruction,
            Instruction::SkipNotEqValueWithRegisterContents {
                register: 0x1,
                value: 0x23
            }
        ));

        let instruction = emulator.fetch();
        assert!(matches!(
            instruction,
            Instruction::SkipEqRegisters {
                register_x: 0x1,
                register_y: 0x2
            }
        ));

        let instruction = emulator.fetch();
        assert!(matches!(
            instruction,
            Instruction::SkipNotEqRegisters {
                register_x: 0x1,
                register_y: 0x2
            }
        ));
    }
}
