use std::{
    sync::{
        Arc,
        atomic::{AtomicU16, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
// NOTE: The test roms were found at https://github.com/Timendus/chip8-test-suite

use crate::{
    ApplicationError, BASE_SCREEN_HEIGHT, BASE_SCREEN_WIDTH, Draw, ReadInputState,
    emulator::{
        fonts::{BIG_FONT, BIG_FONT_ADDR, FONT, FONT_ADDR},
        instructions::{Instruction, KeyStateToCheck},
        logical_operator::{Direction, LogicalOperator},
        quirks::{QuirksFields, QuirksMode},
        sub_commands::{FontVariant, SubCommand},
    },
    twister_rand::MarsenneTwister32,
    utils::u8_to_arr,
};

/// This is the main emulator interface, used to configure and run the interpreter.
#[derive(Debug)]
pub struct Emulator<T: Draw, P: ReadInputState> {
    memory: [u8; 0x1000],
    stack: Vec<usize>,
    variable_registers: [u8; 16],
    screen_buffer: Vec<u8>,
    index_register: usize,
    program_counter: usize,
    delay_timer: Arc<AtomicU16>,
    sound_timer: Arc<AtomicU16>,
    drawer: T,
    input_provider: P,
    max_draw_delay: Duration,
    last_draw: SystemTime,
    quirks: QuirksFields,
    screen_width: usize,
    screen_height: usize,
    rng: MarsenneTwister32,
}

impl<T: Draw, P: ReadInputState> Emulator<T, P> {
    /// Init could I guess also have been named 'new()', but it just returns an instance of the
    /// interpreter/emulator.
    pub fn init(program: Vec<u8>, drawer: T, input_provider: P) -> Result<Self, ApplicationError> {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Now is always after UNIX Epoch");

        let mut emulator = Self {
            memory: [0; 0x1000],
            stack: vec![],
            variable_registers: [0; 16],
            screen_buffer: vec![0; BASE_SCREEN_WIDTH * BASE_SCREEN_HEIGHT],
            index_register: 0,
            program_counter: 0x200,
            delay_timer: Arc::new(AtomicU16::new(0)),
            sound_timer: Arc::new(AtomicU16::new(0)),
            drawer,
            input_provider,
            max_draw_delay: Duration::from_millis(7),
            last_draw: SystemTime::now(),
            quirks: QuirksMode::Chip8.into(),
            screen_width: 64,
            screen_height: 32,
            rng: MarsenneTwister32::new((seed.as_micros() % u32::MAX as u128) as u32),
        };

        emulator.set_mem_block(&FONT, FONT_ADDR)?;
        emulator.set_mem_block(&BIG_FONT, BIG_FONT_ADDR)?;
        emulator.set_mem_block(&program, 0x200)?;

        let delay_timer = emulator.delay_timer.clone();
        let sound_timer = emulator.sound_timer.clone();
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

        Ok(emulator)
    }

    /// Run the emulator/interpreter on the current thread in a busy loop.
    /// The drawing speed is purely determined by the max draw delay
    pub fn run_blocking(&mut self) {
        loop {
            let instruction = self.fetch();
            self.execute(instruction);
        }
    }

    /// Reset the emulator's state to its initial state.
    pub fn reset(&mut self, program: Vec<u8>) -> Result<(), ApplicationError> {
        self.memory = [0; 0x1000];
        self.stack = vec![];
        self.variable_registers = [0; 16];
        self.screen_buffer = vec![0; BASE_SCREEN_WIDTH * BASE_SCREEN_HEIGHT];
        self.index_register = 0;
        self.program_counter = 0x200;
        self.max_draw_delay = Duration::from_millis(7);
        self.last_draw = SystemTime::now();
        self.quirks = QuirksMode::Chip8.into();
        self.screen_width = 64;
        self.screen_height = 32;
        self.set_mem_block(&program, 0x200)?;
        Ok(())
    }

    /// Set the quirks mode to a predefined behaviour (I'm not sure how consistent this is)
    pub fn set_quirks_mode(&mut self, quirks_mode: QuirksMode) -> &mut Self {
        self.quirks = quirks_mode.into();
        self
    }

    /// Set only specific flags for quirks to be followed (I'm not sure how consistent this is)
    pub fn with_custom_quirks(&mut self, quirks: QuirksFields) -> &mut Self {
        self.quirks = quirks;
        self
    }

    /// Set the amount of time the interpreter will wait before drawing to the screen. I.E. draw to
    /// screen rate.
    pub fn set_max_draw_delay(&mut self, rate: Duration) -> &mut Self {
        self.max_draw_delay = rate;
        self
    }

    fn fetch(&mut self) -> Instruction {
        let tophalf = self.memory[self.program_counter];
        let bothalf = self.memory[self.program_counter + 1];
        self.program_counter += 2;
        (((tophalf as u16) << 8) | bothalf as u16).into()
    }

    fn set_high_res(&mut self) {
        self.screen_width = BASE_SCREEN_WIDTH << 1;
        self.screen_height = BASE_SCREEN_HEIGHT << 1;
    }

    fn set_low_res(&mut self) {
        self.screen_width = BASE_SCREEN_WIDTH;
        self.screen_height = BASE_SCREEN_HEIGHT;
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
                if self.quirks.jumping {
                    self.program_counter = address + self.variable_registers[register_x] as usize;
                } else {
                    self.program_counter = address + self.variable_registers[0] as usize;
                }
            }
            Instruction::Random {
                register_x,
                val_to_and,
            } => {
                let randint = (self.rng.generate() % 255) as u8;
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
            Instruction::SubCommand { register, command } => {
                self.perform_sub_command(command, register)
            }
            Instruction::SkipIfKey {
                register,
                state_to_check,
            } => {
                self.skip_if_key(state_to_check, register);
            }
            Instruction::SetHiRes => {
                self.set_high_res();
                self.screen_buffer
                    .resize(self.screen_width * self.screen_height, 0);
                self.wait_to_display();
                self.drawer
                    .draw_buffer(&self.screen_buffer, self.screen_width, self.screen_height);
            }
            Instruction::SetLoRes => {
                self.set_low_res();
                self.screen_buffer
                    .resize(self.screen_width * self.screen_height, 0);
                self.wait_to_display();
                self.drawer
                    .draw_buffer(&self.screen_buffer, self.screen_width, self.screen_height);
            }
            Instruction::ScrollDown { amount } => {
                let px_to_shift = self.screen_width * amount as usize;
                let mut new_buffer = vec![0; px_to_shift];
                new_buffer.extend(&self.screen_buffer);
                new_buffer.resize(self.screen_width * self.screen_height, 0);
                self.screen_buffer = new_buffer;
            }
            Instruction::ScrollRight => {
                let new_buffer: Vec<u8> = self
                    .screen_buffer
                    .chunks(self.screen_width)
                    .flat_map(|x| {
                        let mut row = vec![0, 0, 0, 0];
                        row.extend(x);
                        row.resize(self.screen_width, 0);
                        row
                    })
                    .collect();
                self.screen_buffer = new_buffer;
            }
            Instruction::ScrollLeft => {
                let new_buffer: Vec<u8> = self
                    .screen_buffer
                    .chunks(self.screen_width)
                    .flat_map(|x| {
                        let mut row = x[4..].to_vec();
                        row.resize(self.screen_width, 0);
                        row
                    })
                    .collect();
                self.screen_buffer = new_buffer;
            }
            Instruction::Unimplemented(val) => {
                eprintln!("UNIMPLEMENTED: {}", val)
            }
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
                KeyStateToCheck::Invalid => {}
            }
        };
    }

    fn perform_sub_command(&mut self, command: SubCommand, register: usize) {
        match command {
            SubCommand::ReadDelayTimer => {
                self.variable_registers[register] = self.delay_timer.load(Ordering::Relaxed) as u8;
            }
            SubCommand::SetDelayTimer => {
                self.delay_timer
                    .store(self.variable_registers[register] as u16, Ordering::Relaxed);
            }
            SubCommand::SetSoundTimer => {
                self.sound_timer
                    .store(self.variable_registers[register] as u16, Ordering::Relaxed);
            }
            SubCommand::AddToIndexRegister => {
                self.index_register += self.variable_registers[register] as usize;
            }
            SubCommand::GetFontCharacter(FontVariant::Small) => {
                self.index_register =
                    FONT_ADDR + (self.variable_registers[register] & 0xF) as usize * 5;
            }
            SubCommand::GetFontCharacter(FontVariant::Big) => {
                self.index_register =
                    BIG_FONT_ADDR + (self.variable_registers[register]) as usize * 10;
            }
            SubCommand::DecimalConversion => {
                let val = self.variable_registers[register];
                let val = u8_to_arr(val);

                for (i, val) in val.iter().enumerate() {
                    let wrapped = (self.index_register + i) % self.memory.len();
                    if let Some(x) = self.memory.get_mut(wrapped) {
                        *x = *val;
                    };
                }
            }
            SubCommand::LoadFrom => {
                for (i, reg) in self.variable_registers[0..=register].iter_mut().enumerate() {
                    let wrapped = (self.index_register + i) % self.memory.len();
                    if let Some(x) = self.memory.get(wrapped) {
                        *reg = *x;
                    }
                }

                if self.quirks.memory {
                    self.index_register += register + 1;
                }
            }
            SubCommand::StoreTo => {
                for (i, reg) in self.variable_registers[0..=register].iter().enumerate() {
                    let wrapped = (self.index_register + i) % self.memory.len();
                    if let Some(x) = self.memory.get_mut(wrapped) {
                        *x = *reg;
                    }
                }
                if self.quirks.memory {
                    self.index_register += register + 1;
                }
            }
            SubCommand::GetKey => {
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
            SubCommand::Unimplemented(_value) => {}
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
                // NOTE: Though this quirk is called shifting, to be more in line with the test
                // suite, this is inverted.
                if !self.quirks.shifting {
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
        let (x_value, y_value) = (
            (self.variable_registers[x_register] % self.screen_width as u8) as u16,
            (self.variable_registers[y_register] % self.screen_height as u8) as u16,
        );
        self.variable_registers[0xF] = 0;

        let start_loc = y_value * self.screen_width as u16 + x_value;

        let (width, height) = { if height == 0 { (16, 16) } else { (8, height) } };

        // For each row in the sprite
        for i in 0..height {
            let mut overdrawn_y = false;
            let sprite_row = if width == 8 {
                let wrapped_mem_index = (self.index_register + i as usize) % self.memory.len();
                let Some(sprite_row) = self.memory.get(wrapped_mem_index) else {
                    break;
                };
                *sprite_row as u16
            } else {
                let wrapped_mem_index_first_half =
                    (self.index_register + (i * 2) as usize) % self.memory.len();
                let Some(sprite_left_half) = self.memory.get(wrapped_mem_index_first_half) else {
                    break;
                };
                let wrapped_mem_index_second_half =
                    (self.index_register + (i * 2 + 1) as usize) % self.memory.len();
                let Some(sprite_right_half) = self.memory.get(wrapped_mem_index_second_half) else {
                    break;
                };
                (*sprite_left_half as u16) << 8 | *sprite_right_half as u16
            };

            let current_loc = start_loc + self.screen_width as u16 * i as u16;

            if y_value + i as u16 > (self.screen_height - 1) as u16 {
                if self.quirks.clipping {
                    continue;
                } else {
                    overdrawn_y = true;
                }
            }
            // For each pixel in the row
            for j in 0..width {
                let mut overdrawn_x = false;
                let mask: u16 = 0b1 << (width - j - 1);

                if (x_value + j) > (self.screen_width - 1) as u16 {
                    if self.quirks.clipping {
                        continue;
                    } else {
                        overdrawn_x = true;
                    }
                }

                let mut to_get_index = current_loc + j;

                // NOTE: Only in Quirks.clipping
                // Because we working with a flattened array, lol
                // subract one row to keep alignment of the px
                if overdrawn_x {
                    to_get_index -= self.screen_width as u16;
                }

                // NOTE: Only in Quirks.clipping
                // If we go off the bottom of the screen, subract an entire screen
                // to get back to the top
                if overdrawn_y {
                    to_get_index -= (self.screen_width * self.screen_height) as u16;
                }

                if let Some(x) = self.screen_buffer.get_mut((to_get_index) as usize) {
                    let pixel = if (mask & sprite_row) > 0 { 1 } else { 0 };
                    // Collision flag if target and px are both on.
                    if pixel > 0 && *x > 0 {
                        self.variable_registers[0xF] = 1;
                    }
                    *x ^= pixel;
                }
            }
        }
        self.wait_to_display();
        self.drawer
            .draw_buffer(&self.screen_buffer, self.screen_width, self.screen_height);
    }

    fn wait_to_display(&mut self) {
        let now = SystemTime::now();
        let time_since_last_draw = now
            .duration_since(self.last_draw)
            .expect("Earlier is before now.");

        if time_since_last_draw < self.max_draw_delay {
            std::thread::sleep(self.max_draw_delay - time_since_last_draw);
        }
        self.last_draw = SystemTime::now();
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
}

/// Data that is returned after each iteration of the interpreter as long as it is running as an
/// iterator.
#[derive(Debug)]
pub struct EmulatorState {
    pub memory: [u8; 0x1000],
    pub stack: Vec<usize>,
    pub variable_registers: [u8; 16],
    pub screen_buffer: Vec<u8>,
    pub index_register: usize,
    pub program_counter: usize,
    pub last_instruction: Instruction,
}

impl<T: Draw, P: ReadInputState> Iterator for Emulator<T, P> {
    type Item = EmulatorState;

    fn next(&mut self) -> Option<Self::Item> {
        let instruction = self.fetch();
        self.execute(instruction.clone());
        Some(EmulatorState {
            last_instruction: instruction,
            memory: self.memory,
            stack: self.stack.clone(),
            variable_registers: self.variable_registers,
            screen_buffer: self.screen_buffer.clone(),
            index_register: self.index_register,
            program_counter: self.program_counter,
        })
    }
}

#[cfg(test)]
mod tests {
    struct DummyInput;
    struct DebugDrawer;
    impl Draw for DebugDrawer {
        fn draw_buffer(&mut self, _: &[u8], _: usize, _: usize) {}
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
            .set_mem_block(&FONT, FONT_ADDR)
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
