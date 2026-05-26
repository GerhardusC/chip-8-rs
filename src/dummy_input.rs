use std::collections::HashMap;

use crate::ReadInputState;

impl ReadInputState for DummyInput {
    fn init() -> Self {
        Self
    }

    fn read_keys_state(
        &self,
    ) -> Result<std::collections::HashMap<char, std::time::SystemTime>, String> {
        Ok(HashMap::new())
    }
}
pub struct DummyInput;
