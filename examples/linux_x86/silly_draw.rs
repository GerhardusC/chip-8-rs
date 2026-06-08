use chip_eight::Draw;
use std::io::Write;

pub struct Drawer;

impl Drawer {
    fn return_home(&self) {
        print!("\x1b[H");
        let _ = std::io::stdout().flush();
    }
    pub fn init() -> Self {
        println!("\x1b[?1049h");
        let x = Self;
        x.return_home();
        x
    }
}

impl Draw for Drawer {
    fn draw_buffer(&mut self, screen_buf: &[u8], screen_width: usize, _screen_height: usize) {
        println!("{}", create_buffer_string(screen_buf, screen_width));
        self.return_home();
    }

    fn clear_screen(&mut self) {
        self.return_home();
        let x = (" ".repeat(128) + "\n").repeat(128);

        println!("{x}");
        self.return_home();
    }
}

fn create_buffer_string(buf: &[u8], screen_width: usize) -> String {
    buf.chunks(screen_width)
        .map(|chars| {
            chars
                .iter()
                .map(|c| if *c > 0 { '█' } else { ' ' })
                .collect()
        })
        .collect::<Vec<String>>()
        .join("\n")
}

impl Drop for Drawer {
    fn drop(&mut self) {
        println!("\x1b[?1049l");
    }
}
