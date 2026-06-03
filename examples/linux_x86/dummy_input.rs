use crate::ReadInputState;

impl ReadInputState for DummyInput {
    fn read_keys_state(&self) -> Result<[u8; 16], String> {
        Ok([0; 16])
    }

    fn reset_keys_state(&mut self) {}
}
pub struct DummyInput;
