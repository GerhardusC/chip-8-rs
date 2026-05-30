
#[derive(Debug)]
pub enum Instruction {
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
