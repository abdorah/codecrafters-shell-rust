//! Input handling and line editing functionality
//! 
//! This module provides cross-platform input handling for the shell, including:
//! - Key reading from terminal input
//! - Line editing with cursor movement
//! - Platform-specific implementations for Unix and Windows
//! 
//! The module is organized into:
//! - [`constants`]: Key code definitions
//! - [`lib`]: Line editor implementation  
//! - Platform-specific key reading (unix.rs/windows.rs)

pub mod constants;
pub mod lib;

#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

#[cfg(unix)]
pub use unix::read_key;
#[cfg(windows)]
pub use windows::read_key;
