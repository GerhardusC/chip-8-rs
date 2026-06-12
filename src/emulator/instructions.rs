use crate::emulator::{logical_operator::LogicalOperator, sub_commands::SubCommand};

impl From<u16> for Instruction {
    fn from(value: u16) -> Self {
        match value >> 12 {
            0x0 if value == 0x00EE => Self::Return,
            0x0 if value == 0x00E0 => Self::ClearScreen,
            0x0 if value == 0x00FF => Self::SetHiRes,
            0x0 if value == 0x00FE => Self::SetLoRes,
            0x0 if value == 0x00FB => Self::ScrollRight,
            0x0 if value == 0x00FC => Self::ScrollLeft,
            0x0 if value & 0x00F0 == 0x00C0 => Self::ScrollDown {
                amount: (value & 0xF) as u8,
            },
            0x0 => Self::Unimplemented(value),
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
            0x6 => Self::SetGeneralRegister {
                register: (0xF00 & value) as usize >> 8,
                value: (0xFF & value) as u8,
            },
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
            0xA => Self::SetIndexRegister(0xFFF & value),
            0xB => Self::JumpWithOffset {
                register_x: (value & 0x0F00) as usize >> 8,
                address: (value & 0x0FFF) as usize,
            },
            0xC => Self::Random {
                register_x: (value & 0x0F00) as usize >> 8,
                val_to_and: (value & 0x00FF) as u8,
            },
            0xD => Self::Draw {
                x_register: (0xF00 & value) as usize >> 8,
                y_register: (0xF0 & value) as usize >> 4,
                height: (0xF & value) as u8,
            },
            0xE => Self::SkipIfKey {
                register: (0xF00 & value) as usize >> 8,
                state_to_check: match 0xFF & value {
                    0x9E => KeyStateToCheck::IsPressed,
                    0xA1 => KeyStateToCheck::NotPressed,
                    _ => KeyStateToCheck::Invalid,
                },
            },
            0xF => Self::SubCommand {
                register: (0xF00 & value) as usize >> 8,
                command: SubCommand::from(value),
            },

            _ => unreachable!(
                "By bitshifting the value 12 to the right, we only have 4 bits, i.e. 0x0-0xF as insturctions"
            ),
        }
    }
}
#[derive(Debug, Clone)]
pub enum Instruction {
    ClearScreen,
    Return,
    Call(usize),
    Jump(u16),
    SetIndexRegister(u16),
    SetGeneralRegister {
        register: usize,
        value: u8,
    },
    AddToRegister {
        register: usize,
        value: u8,
    },
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
    SubCommand {
        register: usize,
        command: SubCommand,
    },
    SkipIfKey {
        register: usize,
        state_to_check: KeyStateToCheck,
    },
    SetHiRes,
    SetLoRes,
    ScrollDown {
        amount: u8,
    },
    ScrollRight,
    ScrollLeft,
    Unimplemented(u16),
}

#[derive(Debug, Clone)]
pub enum KeyStateToCheck {
    IsPressed,
    NotPressed,
    Invalid,
}
