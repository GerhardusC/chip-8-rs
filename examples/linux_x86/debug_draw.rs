use chip_eight::Draw;

pub struct DebugDrawer;

impl Draw for DebugDrawer {
    fn init() -> Self {
        Self
    }

    fn draw_buffer(&self, screen_buf: &[u8]) {
        let x = screen_buf
            .chunks(64)
            .map(|s| s.iter().map(|c| if *c == 0 { ' ' } else { '█' }).collect())
            .collect::<Vec<String>>()
            .join("\n");
        println!("Call to draw_buffer:\n{}", x);
    }

    fn clear_screen(&self) {
        dbg!("Call to clear screen");
    }
}
