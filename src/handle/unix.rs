//! Unix key reading implementation
//! 
//! Provides key reading functionality on Unix-like systems by reading
//! raw bytes from stdin and interpreting them as key codes, including
//! escape sequences for special keys.

use crate::handle::constants::Key;
use std::io;
use std::io::Read;

/// Reads a single key press from stdin on Unix systems
/// 
/// Interprets raw bytes and escape sequences to determine which key was pressed.
/// Handles:
/// - Regular ASCII characters
/// - Control characters (Ctrl+C, Ctrl+D, etc.)
/// - Escape sequences for arrow keys, function keys, etc.
/// 
/// # Returns
/// 
/// - `Ok(Some(Key))` if a key was read and recognized
/// - `Ok(None)` if no input was available
/// - `Err(io::Error)` if a read error occurred
/// 
/// # Note
/// 
/// This function should be called while the terminal is in raw mode
/// for proper key detection.
pub fn read_key() -> io::Result<Option<Key>> {
    let mut stdin = io::stdin();
    let mut buf = [0u8; 1];

    if stdin.read(&mut buf)? == 0 {
        return Ok(None);
    }

    let key = match buf[0] {
        b'\n' | b'\r' => Key::Enter,
        b'\t' => Key::Tab,
        0x7f | 0x08 => Key::Backspace,
        0x03 => Key::CtrlC,
        0x04 => Key::CtrlD,
        0x01 => Key::CtrlA,
        0x05 => Key::CtrlE,
        0x1b => {
            let mut seq = [0u8; 2];
            if stdin.read(&mut seq[0..1])? > 0 && seq[0] == b'[' {
                if stdin.read(&mut seq[1..2])? > 0 {
                    match seq[1] {
                        b'A' => Key::Up,
                        b'B' => Key::Down,
                        b'C' => Key::Right,
                        b'D' => Key::Left,
                        b'H' => Key::Home,
                        b'F' => Key::End,
                        b'3' => {
                            let mut tilde = [0u8; 1];
                            let _ = stdin.read(&mut tilde);
                            Key::Delete
                        }
                        _ => Key::Unknown,
                    }
                } else {
                    Key::Unknown
                }
            } else {
                Key::Unknown
            }
        }
        ch if ch >= 32 && ch < 127 => Key::Char(ch as char),
        _ => Key::Unknown,
    };

    Ok(Some(key))
}
