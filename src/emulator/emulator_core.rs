use std::{
    sync::{
        Arc,
        atomic::{AtomicU16, Ordering},
    },
    time::{Duration, UNIX_EPOCH},
};
// NOTE: The test roms were found at https://github.com/Timendus/chip8-test-suite

use crate::{
    ApplicationError, Draw, ReadInputState, SCREEN_HEIGHT, SCREEN_WIDTH,
    emulator::{
        instructions::{Instruction, KeyStateToCheck},
        logical_operator::{Direction, LogicalOperator},
        quirks::{QuirksFields, QuirksMode},
        sub_commands::FCommand,
    },
    u8_to_arr,
};

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

#[derive(Debug)]
pub struct Emulator<T: Draw, P: ReadInputState> {
    memory: [u8; 0x1000],
    stack: Vec<usize>,
    variable_registers: [u8; 16],
    screen_buffer: [u8; SCREEN_WIDTH * SCREEN_HEIGHT],
    font_addr: usize,
    index_register: usize,
    program_counter: usize,
    delay_timer: Arc<AtomicU16>,
    sound_timer: Arc<AtomicU16>,
    drawer: T,
    input_provider: P,
    tick_rate: Duration,
    quirks: QuirksFields,
}

impl<T: Draw, P: ReadInputState> Emulator<T, P> {
    pub fn init(program: Vec<u8>, drawer: T, input_provider: P) -> Result<Self, ApplicationError> {
        let mut emulator = Self {
            memory: [0; 0x1000],
            stack: vec![],
            variable_registers: [0; 16],
            screen_buffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT],
            font_addr: 0x50,
            index_register: 0,
            program_counter: 0x200,
            delay_timer: Arc::new(AtomicU16::new(0)),
            sound_timer: Arc::new(AtomicU16::new(0)),
            drawer,
            input_provider,
            tick_rate: Duration::from_millis(1),
            quirks: QuirksMode::Chip8.into(),
        };

        emulator.set_mem_block(&program, 0x200)?;
        emulator.set_font(&FONT)?;

        Ok(emulator)
    }

    pub fn set_tick_rate(&mut self, rate: Duration) -> &mut Self {
        self.tick_rate = rate;
        self
    }

    // TODO: See if we need to support custom fonts.
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
                self.clear_screen();
            }
            Instruction::Draw {
                x_register,
                y_register,
                height,
            } => {
                self.draw(x_register, y_register, height);
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
                self.perform_logical_operator(register_x, register_y, operator);
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
            Instruction::FCommand { register, command } => {
                self.perform_f_command(command, register)
            }
            Instruction::SkipIfKey {
                register,
                state_to_check,
            } => {
                self.skip_if_key(state_to_check, register);
            }
            Instruction::Unimplemented(_) => {}
            Instruction::Error(_) => {}
        }
    }

    fn skip_if_key(&mut self, state_to_check: KeyStateToCheck, register: usize) {
        if let Ok(keys) = self.input_provider.read_keys_state() {
            let current_key_state = keys[(self.variable_registers[register] & 0xF) as usize];
            match state_to_check {
                KeyStateToCheck::IsPressed => {
                    if current_key_state > 0 {
                        self.program_counter += 2;
                    }
                }
                KeyStateToCheck::NotPressed => {
                    if current_key_state == 0 {
                        self.program_counter += 2;
                    }
                }
                KeyStateToCheck::Invalid => {
                    eprintln!("Unrecognised instruction.")
                }
            }
        };
    }

    fn perform_f_command(&mut self, command: FCommand, register: usize) {
        match command {
            FCommand::ReadDelayTimer => {
                self.variable_registers[register] = self.delay_timer.load(Ordering::Relaxed) as u8;
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
                self.index_register = self.font_addr + self.variable_registers[register] as usize;
            }
            FCommand::DecimalConversion => {
                let val = self.variable_registers[register];
                let val = u8_to_arr(val);

                for (i, val) in val.iter().enumerate() {
                    self.memory[self.index_register + i] = *val;
                }
            }
            FCommand::LoadFrom => {
                for (i, reg) in self.memory[self.index_register..=(self.index_register + register)]
                    .iter()
                    .enumerate()
                {
                    self.variable_registers[i] = *reg;
                }
                if self.quirks.memory {
                    self.index_register += register + 1;
                }
            }
            FCommand::StoreTo => {
                for (i, reg) in self.variable_registers[0..=register].iter().enumerate() {
                    self.memory[self.index_register + i] = *reg;
                }
                if self.quirks.memory {
                    self.index_register += register + 1;
                }
            }
            FCommand::GetKey => {
                let mut key_pressed = false;
                if let Ok(keys) = self.input_provider.read_keys_state() {
                    for (i, key) in keys.iter().enumerate() {
                        if *key > 0 {
                            self.variable_registers[register] = i as u8;
                            key_pressed = true;
                            break;
                        }
                    }
                }
                if !key_pressed {
                    self.program_counter -= 2;
                } else {
                    self.input_provider.reset_keys_state();
                };
            }
            FCommand::Unimplemented(value) => {
                eprintln!("COMMAND {value} UNIMPLEMENTED");
            }
        }
    }

    fn perform_logical_operator(
        &mut self,
        register_x: usize,
        register_y: usize,
        operator: LogicalOperator,
    ) {
        match operator {
            LogicalOperator::Set => {
                self.variable_registers[register_x] = self.variable_registers[register_y];
            }
            LogicalOperator::BinaryOr => {
                self.variable_registers[register_x] |= self.variable_registers[register_y];
                if self.quirks.vf_reset {
                    self.variable_registers[0xF] = 0;
                }
            }
            LogicalOperator::BinaryAnd => {
                self.variable_registers[register_x] &= self.variable_registers[register_y];
                if self.quirks.vf_reset {
                    self.variable_registers[0xF] = 0;
                }
            }
            LogicalOperator::LogicalXor => {
                self.variable_registers[register_x] ^= self.variable_registers[register_y];
                if self.quirks.vf_reset {
                    self.variable_registers[0xF] = 0;
                }
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
                if self.quirks.shifting {
                    self.variable_registers[register_x] = self.variable_registers[register_y];
                }

                match direction {
                    Direction::Left => {
                        let top = self.variable_registers[register_x] & 0b1000_0000;
                        let res = self.variable_registers[register_x] << 1;
                        self.variable_registers[register_x] = res;
                        if top > 0 {
                            self.variable_registers[0xF] = 1
                        } else {
                            self.variable_registers[0xF] = 0
                        };
                    }
                    Direction::Right => {
                        let bot = self.variable_registers[register_x] & 0b1;
                        let res = self.variable_registers[register_x] >> 1;
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

    fn draw(&mut self, x_register: usize, y_register: usize, height: u8) {
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
                    if *x > 0 && ans > 0 {
                        *x = 0;
                        self.variable_registers[0xF] = 1;
                    } else if *x == 0 && ans > 0 {
                        self.variable_registers[0xF] = 1;
                        *x = if ans > 0 { 1 } else { 0 };
                    }
                }
            }
        }
        self.drawer.draw_buffer(&self.screen_buffer);
    }

    fn clear_screen(&mut self) {
        for i in self.screen_buffer.iter_mut() {
            *i = 0;
        }
        self.drawer.clear_screen();
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

    pub fn debug(&mut self) {
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

    pub fn run(&mut self) {
        let delay_timer = self.delay_timer.clone();
        let sound_timer = self.sound_timer.clone();
        std::thread::spawn(move || {
            loop {
                let old_val = delay_timer.load(Ordering::Relaxed);
                if old_val > 0 {
                    delay_timer.store(old_val - 1, Ordering::Relaxed);
                }

                let old_val = sound_timer.load(Ordering::Relaxed);
                if old_val > 0 {
                    sound_timer.store(old_val - 1, Ordering::Relaxed);
                }
                std::thread::sleep(Duration::from_millis(6));
            }
        });
        loop {
            let instruction = self.fetch();
            self.execute(instruction);
            std::thread::sleep(self.tick_rate);
        }
    }
}

#[cfg(test)]
mod tests {
    struct DummyInput;
    struct DebugDrawer;
    impl Draw for DebugDrawer {
        fn draw_buffer(&mut self, _screen_buf: &[u8]) {}
        fn clear_screen(&mut self) {}
    }
    impl ReadInputState for DummyInput {
        fn read_keys_state(&self) -> Result<[u8; 16], String> {
            Ok([0; 16])
        }
        fn reset_keys_state(&mut self) {}
    }

    use super::*;

    #[test]
    fn it_can_fetch_an_instruction() {
        let mut emulator =
            Emulator::<DebugDrawer, DummyInput>::init(vec![], DebugDrawer, DummyInput)
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
