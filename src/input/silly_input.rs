use std::{
    collections::HashMap,
    error::Error,
    io::Read,
    sync::{
        Arc, RwLock,
        mpsc::{self},
    },
    thread::JoinHandle,
    time::{Duration, SystemTime},
};

use crate::ReadInputState;

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
struct Termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_line: u8,
    c_cc: [u8; 32],
    c_ispeed: u32,
    c_ospeed: u32,
}

unsafe extern "C" {
    fn tcgetattr(fd: i32, termios_pointer: *mut Termios) -> i32;
    fn tcsetattr(fd: i32, optional_flags: i32, termios_pointer: *const Termios) -> i32;
}

pub fn convert_keymap(max_delay: Duration, key_map: &HashMap<char, SystemTime>) -> [u8; 16] {
    let mut res = [
        b'1', b'2', b'3', b'4', b'q', b'w', b'e', b'r', b'a', b's', b'd', b'f', b'z', b'x', b'c',
        b'v',
    ];
    let now = SystemTime::now();

    for ch in res.iter_mut() {
        if let Some(timestamp) = key_map.get(&(char::from(*ch)))
            && now
                .duration_since(*timestamp)
                .expect("Now is definitely after earlier.")
                < max_delay
        {
            *ch = 1
        } else {
            *ch = 0
        }
    }

    res
}

#[derive(Debug)]
pub struct InputListener {
    keys: Arc<RwLock<HashMap<char, SystemTime>>>,
    original_state: Termios,
}

impl ReadInputState for InputListener {
    fn init() -> Self {
        let mut original = unsafe { std::mem::zeroed() };

        unsafe {
            tcgetattr(0, &mut original);
        }

        let mut modified = original;

        modified.c_lflag &= !(ICANON | ECHO);
        unsafe {
            tcsetattr(0, TCSANOW, &modified);
        }

        let mut listener = Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            original_state: original,
        };

        let _ = listener.listen().map_err(|e| e.to_string());

        listener
    }

    fn read_keys_state(&self) -> Result<[u8; 16], String> {
        if let Ok(x) = self.keys.read() {
            Ok(convert_keymap(Duration::from_millis(100), &x))
        } else {
            Err("Failed to read keys state".to_string())
        }
    }
}

impl InputListener {
    fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        let mut stdin = std::io::stdin();

        let (internal_tx, internal_rx) = mpsc::channel::<char>();

        let input_sender = internal_tx.clone();
        let _: JoinHandle<Result<(), String>> = std::thread::spawn(move || {
            let mut buf = [0; 1];
            loop {
                stdin.read_exact(&mut buf).map_err(|e| e.to_string())?;
                input_sender
                    .send(buf[0].to_ascii_lowercase().into())
                    .map_err(|e| e.to_string())?;
            }
        });

        let keys = self.keys.clone();
        std::thread::spawn(move || {
            while let Ok(c) = internal_rx.recv() {
                if let Ok(mut keys) = keys.write() {
                    keys.insert(c, SystemTime::now());
                }
            }
        });

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
