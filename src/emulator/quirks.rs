/// Modern or Legacy SuperChip behaviour, I can't get it to work entirely correctly.
#[derive(Debug, Clone)]
pub enum SuperChipBehaviour {
    Modern,
    Legacy,
}

/// The preferred predefined set of behaviours.
#[derive(Debug, Clone)]
pub enum QuirksMode {
    Chip8,
    SuperChip(SuperChipBehaviour),
}

/// A customisable set of behaviours.
#[derive(Debug, Clone)]
pub struct QuirksFields {
    pub vf_reset: bool,
    pub memory: bool,
    // NOTE: I actually don't understand what this is supposed to do
    // I have read https://github.com/Timendus/chip8-test-suite/blob/main/legacy-superchip.md
    // but don't seem to be getting it.
    pub disp_wait: bool,
    pub clipping: bool,
    pub shifting: bool,
    pub jumping: bool,
}

impl From<QuirksMode> for QuirksFields {
    fn from(value: QuirksMode) -> Self {
        match value {
            QuirksMode::Chip8 => QuirksFields {
                vf_reset: true,
                memory: true,
                disp_wait: true,
                clipping: true,
                shifting: false,
                jumping: false,
            },
            QuirksMode::SuperChip(super_chip_behaviour) => match super_chip_behaviour {
                SuperChipBehaviour::Modern => QuirksFields {
                    vf_reset: false,
                    memory: false,
                    disp_wait: true,
                    clipping: true,
                    shifting: true,
                    jumping: true,
                },
                SuperChipBehaviour::Legacy => QuirksFields {
                    vf_reset: false,
                    memory: false,
                    disp_wait: false,
                    clipping: true,
                    shifting: true,
                    jumping: true,
                },
            },
        }
    }
}
