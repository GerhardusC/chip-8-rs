use std::io::Write;

use crate::{Draw, SCREEN_HEIGHT, SCREEN_WIDTH};

pub struct Drawer;

impl Draw for Drawer {
    fn init() -> Self {
        println!("\x1b[?1049h");
        let x = Self;
        x.return_home();
        x
    }

    fn draw_buffer(&self, screen_buf: &[u8]) {
        println!("{}", create_buffer_string(screen_buf, SCREEN_WIDTH));
        self.return_home();
    }

    fn return_home(&self) {
        print!("\x1b[H");
        let _ = std::io::stdout().flush();
    }

    fn clear_screen(&self) {
        self.return_home();
        let x = (" ".repeat(SCREEN_WIDTH * 5) + "\n").repeat(SCREEN_HEIGHT * 5);

        println!("{x}");
        self.return_home();
    }
}

fn create_buffer_string(buf: &[u8], screen_width: usize) -> String {
    buf.chunks(screen_width).map(|chars| {
        chars.iter().map(|c| {
            if *c > 0 {'█'} else {' '}
        }).collect()
    }).collect::<Vec<String>>().join("\n")
}

impl Drop for Drawer {
    fn drop(&mut self) {
        println!("\x1b[?1049l");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_create_correct_buffered_string() {
        let b = [
            0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0,
            1, 0, 0, 0, 1, 0,
        ];
        let expected = " █   █ 
  █ █  
   █   
  █ █  
 █   █ ";

        let s = create_buffer_string(&b, 7);
        assert_eq!(s, expected);
    }
}
