//! Windows terminal raw mode implementation
//! 
//! Provides terminal raw mode control on Windows using the Console API.
//! Raw mode disables line input and echo, allowing the shell to
//! process individual key presses immediately.

use std::io;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Console::{
    GetConsoleMode, GetStdHandle, SetConsoleMode, CONSOLE_MODE, ENABLE_ECHO_INPUT,
    ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, STD_INPUT_HANDLE,
};

/// RAII wrapper for terminal raw mode on Windows systems
/// 
/// When created, puts the console into raw mode. When dropped,
/// automatically restores the original console settings.
/// 
/// # Example
/// 
/// ```rust
/// use shell::terminal::RawMode;
/// 
/// {
///     let _raw = RawMode::enable()?;
///     // Console is now in raw mode
///     // Read individual key presses here
/// } // Console automatically restored when _raw is dropped
/// ```
pub struct RawMode {
    /// Handle to the console input
    handle: HANDLE,
    /// Original console mode to restore
    original_mode: CONSOLE_MODE,
}

impl RawMode {
    /// Enables raw mode on the Windows console
    /// 
    /// Disables:
    /// - Line input (ENABLE_LINE_INPUT) - read characters immediately
    /// - Echo input (ENABLE_ECHO_INPUT) - don't automatically print typed characters
    /// 
    /// Enables:
    /// - Processed input (ENABLE_PROCESSED_INPUT) - handle Ctrl+C properly
    /// 
    /// # Returns
    /// 
    /// - `Ok(RawMode)` if raw mode was enabled successfully
    /// - `Err(io::Error)` if console control failed
    pub fn enable() -> io::Result<Self> {
        unsafe {
            let handle = GetStdHandle(STD_INPUT_HANDLE).map_err(io::Error::other)?;

            let mut original_mode = CONSOLE_MODE::default();
            GetConsoleMode(handle, &mut original_mode).map_err(io::Error::other)?;

            let mut new_mode = original_mode;
            new_mode &= !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT);
            new_mode |= ENABLE_PROCESSED_INPUT;

            SetConsoleMode(handle, new_mode).map_err(io::Error::other)?;

            Ok(RawMode {
                handle,
                original_mode,
            })
        }
    }
}

impl Drop for RawMode {
    /// Automatically restores original console settings when dropped
    fn drop(&mut self) {
        unsafe {
            let _ = SetConsoleMode(self.handle, self.original_mode);
        }
    }
}
