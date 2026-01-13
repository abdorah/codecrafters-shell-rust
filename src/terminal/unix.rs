//! Unix terminal raw mode implementation
//! 
//! Provides terminal raw mode control on Unix-like systems using termios.
//! Raw mode disables line buffering and echo, allowing the shell to
//! process individual key presses immediately.

use libc::{c_int, termios, ECHO, ICANON, TCSANOW, VMIN, VTIME};
use std::io;
use std::os::unix::io::AsRawFd;

/// RAII wrapper for terminal raw mode on Unix systems
/// 
/// When created, puts the terminal into raw mode. When dropped,
/// automatically restores the original terminal settings.
/// 
/// # Example
/// 
/// ```rust
/// use shell::terminal::RawMode;
/// 
/// {
///     let _raw = RawMode::enable()?;
///     // Terminal is now in raw mode
///     // Read individual key presses here
/// } // Terminal automatically restored when _raw is dropped
/// ```
pub struct RawMode {
    /// File descriptor for stdin
    fd: c_int,
    /// Original terminal settings to restore
    original: termios,
}

impl RawMode {
    /// Enables raw mode on the terminal
    /// 
    /// Disables:
    /// - Line buffering (ICANON) - read characters immediately
    /// - Echo (ECHO) - don't automatically print typed characters
    /// 
    /// Sets:
    /// - VMIN = 0 - don't wait for minimum characters
    /// - VTIME = 1 - timeout after 0.1 seconds
    /// 
    /// # Returns
    /// 
    /// - `Ok(RawMode)` if raw mode was enabled successfully
    /// - `Err(io::Error)` if terminal control failed
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
    /// Automatically restores original terminal settings when dropped
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(self.fd, TCSANOW, &self.original);
        }
    }
}
