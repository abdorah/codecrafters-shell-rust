/// ============================================
/// TERMINAL RAW MODE - UNIX
/// ============================================
use libc::{ECHO, ICANON, TCSANOW, VMIN, VTIME, c_int, termios};
use std::io;
use std::io::Read;
use std::os::unix::io::AsRawFd;

pub struct RawMode {
    fd: c_int,
    original: termios,
}

impl RawMode {
    pub fn enable() -> io::Result<Self> {
        let fd = io::stdin().as_raw_fd();
        let mut original = unsafe { std::mem::zeroed() };

        if unsafe { libc::tcgetattr(fd, &mut original) } != 0 {
            return Err(io::Error::last_os_error());
        }

        let mut raw = original;
        raw.c_lflag &= !(ICANON | ECHO);
        raw.c_cc[VMIN] = 0;
        raw.c_cc[VTIME] = 1;

        if unsafe { libc::tcsetattr(fd, TCSANOW, &raw) } != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(RawMode { fd, original })
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(self.fd, TCSANOW, &self.original);
        }
    }
}
