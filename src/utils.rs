fn pad_with_leading_zeroes(full_len: usize, input: &str) -> String {
    let pad_len = full_len - input.len();
    let mut x = String::with_capacity(full_len);
    for _ in 0..pad_len {
        x.push('0');
    }
    x += input;
    x
}

pub fn u8_to_arr(input: u8) -> [u8; 3] {
    let input = input.to_string();
    let items = pad_with_leading_zeroes(3, &input);

    let items: Vec<u8> = items
        .chars()
        .map(|c| c.to_digit(10).unwrap_or(0) as u8)
        .collect();

    let x = items.first().unwrap_or(&0);
    let y = items.get(1).unwrap_or(&0);
    let z = items.get(2).unwrap_or(&0);

    [*x, *y, *z]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_u8_to_arr() {
        assert_eq!(u8_to_arr(132), [1, 3, 2]);
        assert_eq!(u8_to_arr(32), [0, 3, 2]);
        assert_eq!(u8_to_arr(204), [2, 0, 4]);
        assert_eq!(u8_to_arr(2), [0, 0, 2]);
    }
}
