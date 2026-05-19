use crate::Draw;

pub struct DebugDrawer;

impl Draw for DebugDrawer {
    fn init() -> Self {
        Self
    }

    fn draw_buffer(&self, screen_buf: &[u8]) {
        let x = screen_buf
            .chunks(64)
            .map(|s| format!("{:?}", s))
            .collect::<Vec<String>>()
            .join("\n");
        println!("Call to draw_buffer:\n{}", x);
    }

    fn return_home(&self) {
        dbg!("Call to return home draw_buffer");
    }

    fn clear_screen(&self) {
        dbg!("Call to clear screen");
    }
}
