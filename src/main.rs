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
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

struct Emulator {
    memory: [u8; 0x1000],
    stack: Vec<usize>,
    variable_registers: [u8; 16],
    screen_buffer: [u8; 8*32],
    font_addr: usize,
    index_register: usize,
    program_counter: usize,
    delay_timer: AtomicU16,
    sound_timer: AtomicU16,
}

impl Emulator {
    fn init() -> Result<Self, ApplicationError> {
        let mut emulator = Self {
            memory: [0; 0x1000],
            stack: vec![],
            variable_registers: [0; 16],
            screen_buffer: [0; 8*32],
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

    fn set_program_counter(&mut self, pos: usize) {
        // TODO: Maybe bounds check
        self.program_counter = pos;
    }

    fn set_index_register(&mut self, pos: usize) {
        // TODO: Maybe bounds check
        self.index_register = pos;
    }


    fn fetch(&mut self) -> u16 {
        let tophalf = self.memory[self.program_counter];
        let bothalf = self.memory[self.program_counter + 1];
        self.program_counter += 2;
        ((tophalf as u16) << 8) | bothalf as u16
    }

    fn decode(&mut self, instruction: u16) {


    }

    fn execute(&mut self, instruction: u16) {

    }

    fn set_mem_block(&mut self, set: &[u8], start_addr: usize) -> Result<(), ApplicationError> {
        let end_addr = start_addr + set.len();
        if end_addr > self.memory.len() {
            return Err(ApplicationError::MemoryLocationOutOfRange { max_addr: self.memory.len() - set.len() });
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
        std::thread::scope(|s| {

        });
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut emulator = Emulator::init()?;
    let instruction = emulator.fetch();
    assert_eq!(instruction, 0b1111000010010000);

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
            ApplicationError::MemoryLocationOutOfRange { max_addr} => {
                write!(f, "Tried to write memory out of range, max address: {max_addr}")
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_fetch_an_instruction() {
        let mut emulator = Emulator::init()
            .expect("All initial memory is in range");

        emulator.set_font(&FONT)
            .expect("Should be able to set font");

        emulator.set_program_counter(emulator.font_addr);

        let instruction = emulator.fetch();
        assert_eq!(instruction, 0b1111000010010000);
    }

}
