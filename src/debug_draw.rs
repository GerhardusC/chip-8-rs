use crate::Draw;

pub struct DebugDrawer;

impl Draw for DebugDrawer {
    fn init() -> Self {
        Self
    }

    fn draw_buffer(&self, screen_buf: &[u8]) {
        dbg!(format!("Call to draw_buffer: {:?}", screen_buf));
    }

    fn return_home(&self) {
        dbg!("Call to return home draw_buffer");
    }

    fn clear_screen(&self) {
        dbg!("Call to clear screen");
    }
}
