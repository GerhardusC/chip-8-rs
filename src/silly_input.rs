use std::{
    collections::HashMap,
    error::Error,
    io::Read,
    sync::{
        Arc,
        atomic::AtomicBool,
        mpsc::{self, Sender},
    },
    thread::JoinHandle,
    time::{Duration, SystemTime},
};

const ICANON: u32 = 0x00000002;
const ECHO: u32 = 0x00000008;
const TCSANOW: i32 = 0;

// NOTE: From C termios
// struct termios
//   {
//     tcflag_t c_iflag;		/* input mode flags */
//     tcflag_t c_oflag;		/* output mode flags */
//     tcflag_t c_cflag;		/* control mode flags */
//     tcflag_t c_lflag;		/* local mode flags */
//     cc_t c_line;			/* line discipline */
//     cc_t c_cc[NCCS];		        /* control characters */
//     speed_t c_ispeed;		/* input speed */
//     speed_t c_ospeed;		/* output speed */
//   };

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Termios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_line: u8,
    pub c_cc: [u8; 32],
    pub c_ispeed: u32,
    pub c_ospeed: u32,
}

unsafe extern "C" {
    fn tcgetattr(fd: i32, termios_pointer: *mut Termios) -> i32;
    fn tcsetattr(fd: i32, optional_flags: i32, termios_pointer: *const Termios) -> i32;
}

#[derive(Debug)]
pub enum InputBroadcastEvent {
    KeyState(HashMap<char, SystemTime>),
    Close,
}

pub enum InputEvent {
    Broadcast,
    Character(char),
    Close,
}

pub struct InputState {}

#[derive(Debug)]
pub struct InputListener {
    max_age: u32,
    max_update_delay: u32,
    state_sender: Sender<InputBroadcastEvent>,
    keys: HashMap<char, SystemTime>,
    internal_event_receiver: mpsc::Receiver<InputEvent>,
    internal_event_sender: mpsc::Sender<InputEvent>,
    original_state: Termios,
    should_stop: Arc<AtomicBool>,
}

impl InputListener {
    pub fn new(tx: Sender<InputBroadcastEvent>) -> Self {
        let mut original = unsafe { std::mem::zeroed() };

        unsafe {
            tcgetattr(0, &mut original);
        }

        let mut modified = original;

        modified.c_lflag &= !(ICANON | ECHO);
        unsafe {
            tcsetattr(0, TCSANOW, &modified);
        }

        let (internal_tx, internal_rx) = mpsc::channel();

        Self {
            original_state: original,
            internal_event_receiver: internal_rx,
            internal_event_sender: internal_tx,
            state_sender: tx,
            keys: HashMap::new(),
            should_stop: Arc::new(AtomicBool::new(false)),
            max_update_delay: 10,
            max_age: 300,
        }
    }

    pub fn max_update_delay(&mut self, max_update_delay: u32) -> &mut Self {
        self.max_update_delay = max_update_delay;
        self
    }

    pub fn max_age(&mut self, max_age: u32) -> &mut Self {
        self.max_age = max_age;
        self
    }

    pub fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        let mut buf = [0; 1];
        let mut stdin = std::io::stdin();

        let input_sender = self.internal_event_sender.clone();
        let _: JoinHandle<Result<(), String>> = std::thread::spawn(move || {
            loop {
                stdin.read_exact(&mut buf).map_err(|e| e.to_string())?;
                // NOTE: Use ESC or CTRL-D to exit
                if buf[0] == 4 || buf[0] == 27 {
                    input_sender
                        .send(InputEvent::Close)
                        .map_err(|e| e.to_string())?;
                    return Ok(());
                }
                input_sender
                    .send(InputEvent::Character(buf[0].into()))
                    .map_err(|e| e.to_string())?;
            }
        });

        let should_stop = self.should_stop.clone();
        let pulse_sender = self.internal_event_sender.clone();
        let max_update_delay = self.max_update_delay;
        let _: JoinHandle<Result<(), String>> = std::thread::spawn(move || {
            loop {
                pulse_sender
                    .send(InputEvent::Broadcast)
                    .map_err(|e| e.to_string())?;
                std::thread::sleep(Duration::from_millis(max_update_delay.into()));
                if should_stop.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
            }
            Ok(())
        });

        // Main listern loop
        while let Ok(e) = self.internal_event_receiver.recv() {
            match e {
                InputEvent::Broadcast => {
                    let mut chars = vec![];
                    for (c, last_seen) in self.keys.iter() {
                        let now = SystemTime::now();
                        let age = now.duration_since(*last_seen)?;
                        if age.as_millis() > self.max_age.into() {
                            chars.push(*c);
                        }
                    }
                    // Separate loop, I know, but I can't mutate the thing I'm looping over, which
                    // kinda makes sense tbh
                    for x in chars {
                        self.keys.remove(&x);
                    }

                    if let Err(e) = self
                        .state_sender
                        .send(InputBroadcastEvent::KeyState(self.keys.clone()))
                    {
                        dbg!(e);
                    };
                }
                InputEvent::Character(c) => {
                    dbg!(c);
                    self.keys.insert(c, SystemTime::now());
                }
                InputEvent::Close => {
                    dbg!("CLOSE EVENT FIRED IN MAIN LISTERN LOOP");
                    break;
                }
            }
        }

        Ok(())
    }
}

impl Drop for InputListener {
    fn drop(&mut self) {
        let _ = self.internal_event_sender.send(InputEvent::Close);
        // TODO: Check if I should do some sort of enum for the "state sender" to do cleanup, ie.
        // be able to send close event.
        let _ = self.state_sender.send(InputBroadcastEvent::Close);
        self.should_stop
            .store(true, std::sync::atomic::Ordering::Relaxed);
        unsafe {
            tcsetattr(0, TCSANOW, &self.original_state);
        }
    }
}
