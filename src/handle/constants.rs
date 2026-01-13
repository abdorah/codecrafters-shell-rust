//! Key code constants and definitions
//! 
//! Defines the Key enum that represents all possible key inputs
//! that the shell can handle, including special keys like arrows,
//! control keys, and regular characters.

/// Represents different types of key input
/// 
/// This enum covers all the key types that the shell needs to handle
/// for line editing and command input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
    /// A regular printable character
    Char(char),
    /// Backspace key
    Backspace,
    /// Delete key  
    Delete,
    /// Enter/Return key
    Enter,
    /// Tab key
    Tab,
    /// Left arrow key
    Left,
    /// Right arrow key
    Right,
    /// Up arrow key (for future history support)
    Up,
    /// Down arrow key (for future history support)
    Down,
    /// Home key (beginning of line)
    Home,
    /// End key (end of line)
    End,
    /// Ctrl+C (interrupt)
    CtrlC,
    /// Ctrl+D (EOF)
    CtrlD,
    /// Ctrl+A (beginning of line)
    CtrlA,
    /// Ctrl+E (end of line)
    CtrlE,
    /// Unknown or unsupported key
    Unknown,
}
