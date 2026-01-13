//! Terminal control functionality
//! 
//! This module provides cross-platform terminal raw mode control,
//! allowing the shell to read individual key presses without waiting
//! for Enter and without echoing characters to the screen.
//! 
//! The module provides platform-specific implementations:
//! - Unix: Uses termios for terminal control
//! - Windows: Uses Windows Console API

#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

#[cfg(unix)]
pub use unix::RawMode;
#[cfg(windows)]
pub use windows::RawMode;
