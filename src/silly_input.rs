use std::{
    collections::HashMap,
    error::Error,
    io::Read,
    sync::{
        Arc,
        atomic::AtomicBool,
        mpsc::{self, Receiver, Sender},
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

#[derive(Debug)]
pub struct InputListener {
    internal_event_receiver: mpsc::Receiver<InputEvent>,
    internal_event_sender: mpsc::Sender<InputEvent>,
    keys: HashMap<char, SystemTime>,
    max_age: Option<u32>,
    original_state: Termios,
    pulse_delay: Option<u32>,
    should_stop: Arc<AtomicBool>,
    state_sender: Sender<InputBroadcastEvent>,
}

impl InputListener {
    pub fn init(
        max_update_delay: Option<u32>,
        max_age: Option<u32>,
    ) -> (
        JoinHandle<Result<(), String>>,
        Receiver<InputBroadcastEvent>,
    ) {
        let (tx, rx) = mpsc::channel();

        let tx = tx.clone();
        let jh: JoinHandle<Result<(), String>> = std::thread::spawn(move || {
            let mut input_listener = Self::new(tx);
            if let Some(max_update_delay) = max_update_delay {
                input_listener.set_auto_update(max_update_delay, max_age);
            };
            input_listener.listen().map_err(|e| e.to_string())?;

            Ok(())
        });

        (jh, rx)
    }

    fn new(tx: Sender<InputBroadcastEvent>) -> Self {
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
            internal_event_receiver: internal_rx,
            internal_event_sender: internal_tx,
            keys: HashMap::new(),
            max_age: None,
            original_state: original,
            pulse_delay: None,
            should_stop: Arc::new(AtomicBool::new(false)),
            state_sender: tx,
        }
    }

    fn set_auto_update(&mut self, max_update_delay: u32, max_age: Option<u32>) -> &mut Self {
        self.pulse_delay = Some(max_update_delay);
        self.max_age = max_age;
        self
    }

    fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        let mut buf = [0; 1];
        let mut stdin = std::io::stdin();
        let mut join_handles = vec![];

        let input_sender = self.internal_event_sender.clone();
        let jh: JoinHandle<Result<(), String>> = std::thread::spawn(move || {
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
        join_handles.push(jh);

        // Auto update set TODO: extract to own fn
        if let Some(pulse_delay) = self.pulse_delay {
            let should_stop = self.should_stop.clone();
            let pulse_sender = self.internal_event_sender.clone();
            let jh: JoinHandle<Result<(), String>> = std::thread::spawn(move || {
                loop {
                    pulse_sender
                        .send(InputEvent::Broadcast)
                        .map_err(|e| e.to_string())?;
                    std::thread::sleep(Duration::from_millis(pulse_delay.into()));
                    if should_stop.load(std::sync::atomic::Ordering::Relaxed) {
                        eprintln!("STOPPING PULSE DELAY LOOP");
                        break;
                    }
                }
                Ok(())
            });
            join_handles.push(jh);
        };

        // Main listern loop
        while let Ok(e) = self.internal_event_receiver.recv() {
            match e {
                InputEvent::Broadcast => {
                    // Remove stale values. TODO: Extract to fn
                    let mut to_remove = vec![];
                    if let Some(max_age) = self.max_age {
                        for (c, last_seen) in self.keys.iter() {
                            let now = SystemTime::now();
                            let age = now.duration_since(*last_seen)?;
                            if age.as_millis() > max_age.into() {
                                to_remove.push(*c);
                            }
                        }
                        // Separate loop, I know, but I can't mutate the thing I'm looping over, which
                        // kinda makes sense tbh
                        for x in to_remove {
                            self.keys.remove(&x);
                        }
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
                    eprintln!("CLOSE EVENT FIRED IN MAIN LISTERN LOOP, trying to end other items");
                    let _ = self.state_sender.send(InputBroadcastEvent::Close);
                    self.should_stop
                        .store(true, std::sync::atomic::Ordering::Relaxed);

                    for (i, jh) in join_handles.into_iter().enumerate() {
                        eprintln!("ENDING TASK NUMBER: {}", i + 1);
                        if let Err(e) = jh.join() {
                            dbg!(e);
                        }
                    }
                    break;
                }
            }
        }

        Ok(())
    }
}

impl Drop for InputListener {
    fn drop(&mut self) {
        unsafe {
            tcsetattr(0, TCSANOW, &self.original_state);
        }
    }
}
