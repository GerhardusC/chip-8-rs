use std::{error::Error, io::Read, sync::mpsc, thread::JoinHandle};

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

pub enum InputEvent {
    Character(char),
    Close,
}

#[derive(Debug)]
pub struct InputListener {
    pub event_receiver: mpsc::Receiver<InputEvent>,
    event_sender: mpsc::Sender<InputEvent>,
    original_state: Termios,
}

impl Default for InputListener {
    fn default() -> Self {
        Self::new()
    }
}

impl InputListener {
    pub fn new() -> Self {
        let mut original = unsafe { std::mem::zeroed() };

        unsafe {
            tcgetattr(0, &mut original);
        }

        let mut modified = original;

        modified.c_lflag &= !(ICANON | ECHO);
        unsafe {
            tcsetattr(0, TCSANOW, &modified);
        }

        let (tx, rx) = mpsc::channel();

        Self {
            original_state: original,
            event_receiver: rx,
            event_sender: tx,
        }
    }

    pub fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        let mut buf = [0; 1];
        let mut stdin = std::io::stdin();

        let sender = self.event_sender.clone();
        let _: JoinHandle<Result<(), String>> = std::thread::spawn(move || {
            loop {
                stdin.read_exact(&mut buf).map_err(|e| e.to_string())?;
                // NOTE: Use ESC or CTRL-D to exit
                if buf[0] == 4 || buf[0] == 27 {
                    sender.send(InputEvent::Close).map_err(|e| e.to_string())?;
                    return Ok(());
                }
                sender
                    .send(InputEvent::Character(buf[0].into()))
                    .map_err(|e| e.to_string())?;
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
