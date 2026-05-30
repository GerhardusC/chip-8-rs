#[derive(Debug)]
pub enum LogicalOperator {
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
