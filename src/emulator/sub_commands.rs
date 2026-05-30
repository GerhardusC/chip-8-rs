#[derive(Debug)]
pub enum FCommand {
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
