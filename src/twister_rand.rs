use std::num::Wrapping;

// https://en.wikipedia.org/wiki/Mersenne_Twister
const WORD_SIZE: u32 = 32;
const RECURRENCE: i32 = 624;
const MIDDLE_WORD: usize = 397;
const SEPERATION_POINT: u32 = 31;
const RELATIONAL_NORMAL_COEFFICIENT: u32 = 0x9908B0DF;
const TEMPERING_BITMASK_B: u32 = 0x9D2C5680;
const TEMPERING_BITMASK_C: u32 = 0xEFC60000;
const TEMPERING_BITSHIFT_S: u32 = 7;
const TEMPERING_BITSHIFT_T: u32 = 15;
const ADDITIONAL_SHIFT_U: u32 = 11;
const ADDITIONAL_SHIFT_L: u32 = 18;
const MT19937_F: u32 = 1812433253;
const UMASK: u32 = 0xFFFFFFFF << SEPERATION_POINT;
const LMASK: u32 = 0xFFFFFFFF >> (WORD_SIZE - SEPERATION_POINT);

#[derive(Debug)]
pub struct MarsenneTwister32 {
    state_array: [u32; RECURRENCE as usize],
    x_sub_k: usize,
}

impl MarsenneTwister32 {
    pub fn new(seed: u32) -> Self {
        let mut state_array: [u32; RECURRENCE as usize] = [0; RECURRENCE as usize];
        state_array[0] = seed;

        let mut seed = seed;
        for (i, item) in state_array.iter_mut().enumerate().skip(1) {
            seed = (Wrapping(MT19937_F) * Wrapping(seed ^ (seed >> (WORD_SIZE - 2)))).0 + i as u32;
            *item = seed;
        }

        Self {
            state_array,
            x_sub_k: 0,
        }
    }

    pub fn generate(&mut self) -> u32 {
        let x_sub_n_min_one: i32 = self.x_sub_k as i32 - (RECURRENCE - 1);
        // wrap at recurrence
        let x_sub_n_min_one = if x_sub_n_min_one < 0 {
            x_sub_n_min_one + RECURRENCE
        } else {
            x_sub_n_min_one
        };

        let rhs_transformation: u32 = (self.state_array[self.x_sub_k] & UMASK)
            | (self.state_array[x_sub_n_min_one as usize] & LMASK);

        // xA Trnasformation:
        let x_lower_bit = rhs_transformation & 0b1;
        let x_a: u32 = if x_lower_bit == 0 {
            rhs_transformation >> 1
        } else {
            (rhs_transformation >> 1) ^ RELATIONAL_NORMAL_COEFFICIENT
        };

        let x_sub_n_min_m = self.x_sub_k as i32 - (RECURRENCE - MIDDLE_WORD as i32);
        // wrap at recurrence
        let x_sub_n_min_m = if x_sub_n_min_m < 0 {
            x_sub_n_min_m + RECURRENCE
        } else {
            x_sub_n_min_m
        };

        let outer_transformation = self.state_array[x_sub_n_min_m as usize] ^ x_a; // compute next value in the state
        self.state_array[self.x_sub_k] = outer_transformation;
        self.x_sub_k += 1;

        // wrap at recurrence
        if self.x_sub_k >= RECURRENCE as usize {
            self.x_sub_k = 0
        };

        // tempering
        let mut y = outer_transformation ^ (outer_transformation >> ADDITIONAL_SHIFT_U);
        y = y ^ ((y << TEMPERING_BITSHIFT_S) & TEMPERING_BITMASK_B);
        y = y ^ ((y << TEMPERING_BITSHIFT_T) & TEMPERING_BITMASK_C);
        y ^ (y >> ADDITIONAL_SHIFT_L)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_generate_first_few_random_numbers_correctly() {
        let expected_seq: Vec<u32> = vec![
            3992670690, 3823185381, 1358822685, 561383553, 789925284, 170765737, 878579710,
            3549516158, 2438360421, 2285257250,
        ];

        let mut rng = MarsenneTwister32::new(12345);

        for i in expected_seq {
            assert_eq!(rng.generate(), i);
        }
    }
}
