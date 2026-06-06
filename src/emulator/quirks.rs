#[allow(unused)]
#[derive(Debug)]
pub enum SuperChipBehaviour {
    Modern,
    Legacy,
}

#[allow(unused)]
#[derive(Debug)]
pub enum QuirksMode {
    Chip8,
    SuperChip(SuperChipBehaviour),
    XOChip,
}

#[allow(unused)]
#[derive(Debug)]
pub struct QuirksFields {
    pub vf_reset: bool,
    pub memory: bool,
    pub disp_wait: bool,
    pub clipping: bool,
    pub shifting: bool,
    pub jumping: bool,
}

impl From<QuirksMode> for QuirksFields {
    // TODO: Double check these
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
            QuirksMode::XOChip => QuirksFields {
                vf_reset: false,
                memory: true,
                disp_wait: false,
                clipping: false,
                shifting: false,
                jumping: false,
            },
        }
    }
}
