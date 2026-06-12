#[derive(Debug, Clone)]
pub enum LogicalOperator {
    Set,
    BinaryOr,
    BinaryAnd,
    LogicalXor,
    AddAffectingCarry,
    Subtract,
    SubtractReverse,
    Shift(Direction),
    Invalid,
}

#[derive(Debug, Clone)]
pub enum Direction {
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
