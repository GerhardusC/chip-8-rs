use std::{
    error::Error,
    num::NonZeroU32,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};

use chip_eight::{Draw, Emulator, QuirksMode, ReadInputState, SCREEN_HEIGHT, SCREEN_WIDTH};
use softbuffer::{Context, Surface};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, StartCause, WindowEvent},
    event_loop::{EventLoop, OwnedDisplayHandle},
    keyboard::KeyCode,
    window::Window,
};

// This is just used to scale the resolution. The chip8 interpreter works with a 64x32 px screen by
// default. This will just scale it, e.g. SCALE_FACTOR = 10 -> 640 * 320
const SCALE_FACTOR: usize = 10;

// The emulator needs a handle to drawing to the screen, and one for reading user input. For better
// or worse, I have decided to implement them as traits that need to be implemented for types
// passed to the emulator.
//
// In this implementation is not particularly great or fancy, but it illustrates how the emulator
// will call the requird drawing functions and respond to them.
impl Draw for DisplayOutput {
    // This function will be called by the emulator whenever the draw function is encountered. Its
    // simply a buffer of 1s and 0s.
    fn draw_buffer(&mut self, screen_buf: &[u8]) {
        let _ = self.buffer_sender.send(screen_buf.to_vec());
    }

    fn clear_screen(&mut self) {}
}

impl ReadInputState for KeyboardInput {
    fn read_keys_state(&self) -> Result<[u8; 16], String> {
        let x = std::array::from_fn(|i| self.keys[i].load(Ordering::Relaxed));
        Ok(x)
    }

    fn reset_keys_state(&mut self) {
        for key in self.keys.iter() {
            key.store(0, Ordering::Relaxed);
        }
    }
}

struct KeyboardInput {
    keys: Arc<[AtomicU8; 16]>,
}

struct DisplayOutput {
    buffer_sender: Sender<Vec<u8>>,
}

struct App {
    context: Context<OwnedDisplayHandle>,
    app_state: AppState,
    keys: Arc<[AtomicU8; 16]>,
    buffer_receiver: Receiver<Vec<u8>>,
}

enum AppState {
    Initial,
    Suspended {
        window: Rc<Window>,
    },
    Running {
        surface: Surface<OwnedDisplayHandle, Rc<Window>>,
    },
}

impl ApplicationHandler for App {
    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if let StartCause::Init = cause {
            let window_attrs = Window::default_attributes();
            let window = event_loop
                .create_window(window_attrs)
                .expect("failed creating window");
            let window = Rc::new(window);
            self.app_state = AppState::Suspended { window };
        }
    }
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let AppState::Suspended { window } = &mut self.app_state else {
            eprintln!("Invalid state, resumed when not suspended");
            return;
        };

        let mut surface =
            Surface::new(&self.context, window.clone()).expect("failed creating surface");
        let size = window.inner_size();
        if let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
            if let Err(e) = surface.resize(width, height) {
                dbg!(e);
            };
        }

        self.app_state = AppState::Running { surface };
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let AppState::Running { surface } = &mut self.app_state else {
            eprintln!("Invalid state, app not running");
            return;
        };

        if surface.window().id() != window_id {
            eprintln!("Invalid window");
            return;
        }

        match event {
            WindowEvent::Resized(size) => {
                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    if let Err(e) = surface.resize(width, height) {
                        dbg!(e);
                    };
                }
            }
            WindowEvent::CloseRequested => {
                eprintln!("Close requested");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => match event.physical_key {
                winit::keyboard::PhysicalKey::Code(key_code) => {
                    let key_state = match event.state {
                        ElementState::Pressed => 1,
                        ElementState::Released => 0,
                    };
                    match key_code {
                        KeyCode::KeyX => {
                            self.keys[0].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::Digit1 => {
                            self.keys[1].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::Digit2 => {
                            self.keys[2].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::Digit3 => {
                            self.keys[3].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::KeyQ => {
                            self.keys[4].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::KeyW => {
                            self.keys[5].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::KeyE => {
                            self.keys[6].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::KeyA => {
                            self.keys[7].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::KeyS => {
                            self.keys[8].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::KeyD => {
                            self.keys[9].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::KeyZ => {
                            self.keys[10].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::KeyC => {
                            self.keys[11].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::Digit4 => {
                            self.keys[12].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::KeyR => {
                            self.keys[13].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::KeyF => {
                            self.keys[14].store(key_state, Ordering::Relaxed);
                        }
                        KeyCode::KeyV => {
                            self.keys[15].store(key_state, Ordering::Relaxed);
                        }
                        _ => {}
                    }
                }
                winit::keyboard::PhysicalKey::Unidentified(_native_key_code) => {}
            },
            WindowEvent::RedrawRequested => {
                let _ = surface.resize(
                    NonZeroU32::new((SCREEN_WIDTH * SCALE_FACTOR) as u32)
                        .expect("Definitely non zero"),
                    NonZeroU32::new((SCREEN_HEIGHT * SCALE_FACTOR) as u32)
                        .expect("Definitely non zero"),
                );
                let mut buffer = surface.buffer_mut().expect("Failed to get screen buffer");

                if let Ok(draw_buffer) = self.buffer_receiver.recv_timeout(Duration::from_millis(6))
                {
                    let expanded_buf =
                        expand_buffer(draw_buffer.as_slice(), SCALE_FACTOR as u8, SCREEN_WIDTH);
                    if expanded_buf.len() < buffer.len() {
                        eprintln!("Buffer too big");
                        return;
                    }
                    for (i, x) in buffer.iter_mut().enumerate() {
                        *x = expanded_buf[i];
                    }
                    buffer.present().expect("Failed to present buffer");
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let AppState::Running { surface } = &self.app_state else {
            return;
        };
        surface.window().request_redraw();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args();

    let program_name = if args.len() > 1
        && let Some(arg) = args.last()
    {
        arg
    } else {
        eprintln!(
            "Chip 8 Program needs to be passed as final argument

[USAGE]
cargo run --example winit_example /path/to/chip8/program.c8
"
        );
        std::process::exit(1);
    };

    let Ok(program) = std::fs::read(program_name) else {
        eprintln!("Chip 8 program not found");
        std::process::exit(2);
    };

    let (buffer_sender, buffer_receiver) = mpsc::channel();

    let display_output = DisplayOutput { buffer_sender };

    let keyboard_input = KeyboardInput {
        keys: Arc::new(std::array::from_fn(|_| AtomicU8::new(0))),
    };

    let keys = keyboard_input.keys.clone();

    std::thread::spawn(move || {
        Emulator::init(program, display_output, keyboard_input)
            .expect("Program too large")
            .set_tick_rate(Duration::from_millis(1))
            .set_max_draw_delay(Duration::from_millis(4))
            .set_quirks_mode(QuirksMode::Chip8)
            .run();
    });

    let event_loop = EventLoop::new()?;
    let context = Context::new(event_loop.owned_display_handle())?;
    let mut app = App {
        app_state: AppState::Initial,
        context,
        keys,
        buffer_receiver,
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}

// Really, not the most efficient function, but whatever, I just want to grow pixels by a factor
// and convert 1's to a pixel color and 0's to no pixel color (illustrated by th unit test
// 'it_can_expand_buffer'
fn expand_buffer(buf: &[u8], factor: u8, screen_width: usize) -> Vec<u32> {
    let mut output = vec![];

    for chunk in buf.chunks(screen_width) {
        let mut acc = vec![];
        for c in chunk {
            for _ in 0..factor {
                acc.push(if *c > 0 { 0xFFFFFF00 } else { 0x0 });
            }
        }

        for _ in 0..factor {
            output.extend(acc.as_slice());
        }
    }
    output
}
