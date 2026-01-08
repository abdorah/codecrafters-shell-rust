/// ============================================
/// TERMINAL RAW MODE - WINDOWS
/// ============================================
use std::io;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Console::{
    CONSOLE_MODE, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, GetConsoleMode,
    GetStdHandle, STD_INPUT_HANDLE, SetConsoleMode,
};

pub struct RawMode {
    handle: HANDLE,
    original_mode: CONSOLE_MODE,
}

impl RawMode {
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
    fn drop(&mut self) {
        unsafe {
            let _ = SetConsoleMode(self.handle, self.original_mode);
        }
    }
}
