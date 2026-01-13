use crate::handle::constants::Key;
use std::io::{self};

pub fn read_key() -> io::Result<Option<Key>> {
    use windows::Win32::System::Console::{
        GetStdHandle, INPUT_RECORD, KEY_EVENT, ReadConsoleInputW, STD_INPUT_HANDLE,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        VIRTUAL_KEY, VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_HOME, VK_LEFT, VK_RETURN, VK_RIGHT,
        VK_TAB, VK_UP,
    };

    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE).map_err(io::Error::other)?;

        let mut buffer = [INPUT_RECORD::default()];
        let mut read = 0u32;

        ReadConsoleInputW(handle, &mut buffer, &mut read).map_err(io::Error::other)?;

        if buffer[0].EventType == KEY_EVENT as u16 {
            let event = buffer[0].Event.KeyEvent;

            if !event.bKeyDown.as_bool() {
                return Ok(None);
            }

            let key_code = VIRTUAL_KEY(event.wVirtualKeyCode);
            let char_code = event.uChar.UnicodeChar;
            let ctrl_pressed = event.dwControlKeyState & 0x000F != 0;

            let key = match key_code {
                VK_RETURN => Key::Enter,
                VK_TAB => Key::Tab,
                VK_BACK => Key::Backspace,
                VK_DELETE => Key::Delete,
                VK_LEFT => Key::Left,
                VK_RIGHT => Key::Right,
                VK_UP => Key::Up,
                VK_DOWN => Key::Down,
                VK_HOME => Key::Home,
                VK_END => Key::End,
                _ if ctrl_pressed => match char_code as u8 {
                    3 => Key::CtrlC,
                    4 => Key::CtrlD,
                    1 => Key::CtrlA,
                    5 => Key::CtrlE,
                    _ => Key::Unknown,
                },
                _ => {
                    if char_code > 0 && char_code < 128 {
                        let ch = char::from_u32(char_code as u32).unwrap_or('\0');
                        if ch.is_ascii_graphic() || ch == ' ' {
                            Key::Char(ch)
                        } else {
                            Key::Unknown
                        }
                    } else {
                        Key::Unknown
                    }
                }
            };

            Ok(Some(key))
        } else {
            Ok(None)
        }
    }
}
