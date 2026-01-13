/// ============================================
/// KEY CODES
/// ============================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
    Char(char),
    Backspace,
    Delete,
    Enter,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    CtrlC,
    CtrlD,
    CtrlA,
    CtrlE,
    Unknown,
}
