use std::collections::HashSet;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};

// ============================================
// TERMINAL RAW MODE - UNIX
// ============================================

#[cfg(unix)]
mod terminal {
    use libc::{ECHO, ICANON, TCSANOW, VMIN, VTIME, c_int, termios};
    use std::io;
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
}

// ============================================
// TERMINAL RAW MODE - WINDOWS
// ============================================

#[cfg(windows)]
mod terminal {
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
                let handle = GetStdHandle(STD_INPUT_HANDLE).map_err(|e| io::Error::other(e))?;

                let mut original_mode = CONSOLE_MODE::default();
                GetConsoleMode(handle, &mut original_mode).map_err(|e| io::Error::other(e))?;

                // Disable line input and echo
                let mut new_mode = original_mode;
                new_mode &= !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT);
                new_mode |= ENABLE_PROCESSED_INPUT;

                SetConsoleMode(handle, new_mode).map_err(|e| io::Error::other(e))?;

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
}

// ============================================
// KEY CODES
// ============================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Key {
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

// ============================================
// KEY READER
// ============================================

#[cfg(unix)]
fn read_key() -> io::Result<Option<Key>> {
    let mut stdin = io::stdin();
    let mut buf = [0u8; 1];

    if stdin.read(&mut buf)? == 0 {
        return Ok(None); // Timeout
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
            // Escape sequence
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
                            // Delete key
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

#[cfg(windows)]
fn read_key() -> io::Result<Option<Key>> {
    use windows::Win32::System::Console::{
        GetStdHandle, INPUT_RECORD, KEY_EVENT, ReadConsoleInputW, STD_INPUT_HANDLE,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        VIRTUAL_KEY, VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_HOME, VK_LEFT, VK_RETURN, VK_RIGHT,
        VK_TAB, VK_UP,
    };

    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE).map_err(|e| io::Error::other(e))?;

        let mut buffer = [INPUT_RECORD::default()];
        let mut read = 0u32;

        ReadConsoleInputW(handle, &mut buffer, &mut read).map_err(|e| io::Error::other(e))?;

        if buffer[0].EventType == KEY_EVENT as u16 {
            let event = buffer[0].Event.KeyEvent;

            // Only process key down events
            if !event.bKeyDown.as_bool() {
                return Ok(None);
            }

            let key_code = VIRTUAL_KEY(event.wVirtualKeyCode);
            let char_code = unsafe { event.uChar.UnicodeChar };
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
                _ if ctrl_pressed => {
                    // Handle Ctrl combinations
                    match char_code as u8 {
                        3 => Key::CtrlC, // Ctrl+C
                        4 => Key::CtrlD, // Ctrl+D
                        1 => Key::CtrlA, // Ctrl+A
                        5 => Key::CtrlE, // Ctrl+E
                        _ => Key::Unknown,
                    }
                }
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

// ============================================
// LINE EDITOR
// ============================================

struct LineEditor {
    buffer: String,
    cursor: usize,
}

impl LineEditor {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
        }
    }

    fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    fn insert(&mut self, ch: char) {
        self.buffer.insert(self.cursor, ch);
        self.cursor += 1;
    }

    fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.buffer.remove(self.cursor);
        }
    }

    fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
    }

    fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.cursor < self.buffer.len() {
            self.cursor += 1;
        }
    }

    fn move_home(&mut self) {
        self.cursor = 0;
    }

    fn move_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    fn get_word_at_cursor(&self) -> Option<(usize, usize, &str)> {
        if self.buffer.is_empty() {
            return None;
        }

        let bytes = self.buffer.as_bytes();
        let mut start = self.cursor.min(self.buffer.len().saturating_sub(1));
        let mut end = self.cursor;

        // Find start of word
        while start > 0 && !bytes[start - 1].is_ascii_whitespace() {
            start -= 1;
        }

        // Find end of word
        while end < self.buffer.len() && !bytes[end].is_ascii_whitespace() {
            end += 1;
        }

        if start < end {
            Some((start, end, &self.buffer[start..end]))
        } else {
            None
        }
    }

    fn replace_word(&mut self, start: usize, end: usize, replacement: &str) {
        self.buffer.replace_range(start..end, replacement);
        self.cursor = start + replacement.len();
    }
}

// ============================================
// SHELL STRUCTURES
// ============================================

#[derive(Debug, Clone)]
enum StreamType {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
struct Redirect {
    stream: StreamType,
    file: String,
    append: bool,
}

#[derive(Debug)]
struct ParsedCommand {
    args: Vec<String>,
    redirects: Vec<Redirect>,
}

impl ParsedCommand {
    fn new() -> Self {
        Self {
            args: Vec::new(),
            redirects: Vec::new(),
        }
    }
}

struct Shell {
    paths: Vec<String>,
    builtins: HashSet<&'static str>,
    editor: LineEditor,
}

impl Shell {
    fn new() -> Self {
        Shell {
            paths: Self::parse_path(),
            builtins: HashSet::from(["echo", "exit", "type", "pwd", "cd"]),
            editor: LineEditor::new(),
        }
    }

    fn parse_path() -> Vec<String> {
        let separator = if cfg!(windows) { ';' } else { ':' };

        env::var("PATH")
            .unwrap_or_default()
            .split(separator)
            .map(String::from)
            .collect()
    }

    #[cfg(unix)]
    fn is_executable(path: &Path) -> bool {
        use std::os::unix::fs::PermissionsExt;

        std::fs::metadata(path)
            .map(|m| m.is_file() && (m.permissions().mode() & 0o111 != 0))
            .unwrap_or(false)
    }

    #[cfg(windows)]
    fn is_executable(path: &Path) -> bool {
        path.is_file()
            && path
                .extension()
                .map(|ext| {
                    let ext = ext.to_string_lossy().to_lowercase();
                    matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com" | "ps1")
                })
                .unwrap_or(false)
    }

    fn find_executable(&self, cmd: &str) -> Option<String> {
        #[cfg(windows)]
        let candidates = [
            cmd.to_string(),
            format!("{}.exe", cmd),
            format!("{}.bat", cmd),
            format!("{}.cmd", cmd),
            format!("{}.com", cmd),
        ];

        #[cfg(unix)]
        let candidates = [cmd.to_string()];

        for dir in &self.paths {
            for candidate in &candidates {
                let full_path = Path::new(dir).join(candidate);

                if full_path.exists() && Self::is_executable(&full_path) {
                    return full_path.to_str().map(String::from);
                }
            }
        }
        None
    }

    fn find_completions(&self, partial: &str) -> Vec<String> {
        if partial.is_empty() {
            return Vec::new();
        }

        let mut completions = Vec::new();

        // Check builtins
        for builtin in &self.builtins {
            if builtin.starts_with(partial) {
                completions.push(builtin.to_string());
            }
        }

        // Check executables in PATH
        for dir in &self.paths {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        // Remove extension for comparison on Windows
                        let name_without_ext = if cfg!(windows) {
                            file_name
                                .strip_suffix(".exe")
                                .or_else(|| file_name.strip_suffix(".bat"))
                                .or_else(|| file_name.strip_suffix(".cmd"))
                                .or_else(|| file_name.strip_suffix(".com"))
                                .unwrap_or(&file_name)
                        } else {
                            &file_name
                        };

                        if name_without_ext.starts_with(partial)
                            && Self::is_executable(&entry.path())
                        {
                            completions.push(name_without_ext.to_string());
                        }
                    }
                }
            }
        }

        completions.sort();
        completions.dedup();
        completions
    }

    fn print_prompt(&self) {
        print!("$ ");
        let _ = io::stdout().flush();
    }

    fn redraw_line(&self) {
        print!("\r\x1B[K$ {}", self.editor.buffer);

        // Move cursor to correct position
        let pos = self.editor.cursor;
        let line_len = self.editor.buffer.len();
        if pos < line_len {
            print!("\r\x1B[{}C", pos + 2); // +2 for "$ "
        }

        let _ = io::stdout().flush();
    }

    fn show_completions(&self, completions: &[String]) {
        println!();
        for completion in completions.iter().take(10) {
            println!("  {}", completion);
        }
        if completions.len() > 10 {
            println!("  ... and {} more", completions.len() - 10);
        }
        self.print_prompt();
        print!("{}", self.editor.buffer);
        let _ = io::stdout().flush();
    }

    fn handle_tab(&mut self) {
        if let Some((start, end, word)) = self.editor.get_word_at_cursor() {
            let completions = self.find_completions(word);

            match completions.len() {
                0 => {
                    // No completions - beep
                    print!("\x07");
                    let _ = io::stdout().flush();
                }
                1 => {
                    // Single completion - apply it
                    self.editor.replace_word(start, end, &completions[0]);
                    self.redraw_line();
                }
                _ => {
                    // Multiple completions
                    let common = Self::common_prefix(&completions);
                    if common.len() > word.len() {
                        self.editor.replace_word(start, end, &common);
                        self.redraw_line();
                    } else {
                        self.show_completions(&completions);
                        self.redraw_line();
                    }
                }
            }
        }
    }

    fn common_prefix(strings: &[String]) -> String {
        if strings.is_empty() {
            return String::new();
        }

        let first = &strings[0];
        let mut prefix_len = first.len();

        for s in &strings[1..] {
            prefix_len = first
                .chars()
                .zip(s.chars())
                .take(prefix_len)
                .take_while(|(a, b)| a == b)
                .count();
        }

        first.chars().take(prefix_len).collect()
    }

    fn read_line(&mut self) -> io::Result<bool> {
        use terminal::RawMode;

        self.editor.clear();
        self.print_prompt();

        let _raw = RawMode::enable()?;

        loop {
            match read_key()? {
                None => continue,
                Some(Key::Enter) => {
                    println!();
                    return Ok(true);
                }
                Some(Key::Tab) => self.handle_tab(),
                Some(Key::Backspace) => {
                    self.editor.backspace();
                    self.redraw_line();
                }
                Some(Key::Delete) => {
                    self.editor.delete();
                    self.redraw_line();
                }
                Some(Key::Left) => {
                    self.editor.move_left();
                    self.redraw_line();
                }
                Some(Key::Right) => {
                    self.editor.move_right();
                    self.redraw_line();
                }
                Some(Key::Home) | Some(Key::CtrlA) => {
                    self.editor.move_home();
                    self.redraw_line();
                }
                Some(Key::End) | Some(Key::CtrlE) => {
                    self.editor.move_end();
                    self.redraw_line();
                }
                Some(Key::CtrlC) => {
                    println!("^C");
                    self.editor.clear();
                    return Ok(true);
                }
                Some(Key::CtrlD) => {
                    if self.editor.buffer.is_empty() {
                        println!();
                        return Ok(false);
                    }
                }
                Some(Key::Char(ch)) => {
                    self.editor.insert(ch);
                    self.redraw_line();
                }
                Some(Key::Up) | Some(Key::Down) => {
                    // Could implement history here
                }
                Some(Key::Unknown) => {}
            }
        }
    }

    fn parse(&self) -> (String, ParsedCommand) {
        let parsed = Self::parse_arguments(self.editor.buffer.trim());

        if parsed.args.is_empty() {
            return (String::new(), parsed);
        }

        let command = parsed.args[0].clone();
        let remaining = ParsedCommand {
            args: parsed.args[1..].to_vec(),
            redirects: parsed.redirects,
        };

        (command, remaining)
    }

    fn parse_arguments(input: &str) -> ParsedCommand {
        let mut result = ParsedCommand::new();
        let mut current_arg = String::new();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut chars = input.chars().peekable();

        let mut expecting_file = false;
        let mut current_redirect: Option<Redirect> = None;

        while let Some(c) = chars.next() {
            if expecting_file && !in_single_quote && !in_double_quote {
                match c {
                    ' ' => {
                        if !current_arg.is_empty()
                            && let Some(mut redirect) = current_redirect.take()
                        {
                            redirect.file = current_arg.clone();
                            result.redirects.push(redirect);
                            current_arg.clear();
                            expecting_file = false;
                        }
                        continue;
                    }
                    '\'' => {
                        in_single_quote = true;
                        continue;
                    }
                    '"' => {
                        in_double_quote = true;
                        continue;
                    }
                    _ => {
                        current_arg.push(c);
                        continue;
                    }
                }
            }

            match c {
                '\\' if in_double_quote => {
                    if let Some('"' | '\\' | '$' | '`') = chars.peek() {
                        current_arg.push(chars.next().unwrap());
                    } else {
                        current_arg.push('\\');
                    }
                }

                '\\' if !in_single_quote => {
                    if let Some(next) = chars.next() {
                        current_arg.push(next);
                    }
                }

                '\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                }

                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                }

                '2' if !in_single_quote && !in_double_quote => {
                    if chars.peek() == Some(&'>') {
                        if !current_arg.is_empty() {
                            result.args.push(current_arg.clone());
                            current_arg.clear();
                        }

                        chars.next();
                        let append = chars.peek() == Some(&'>');
                        if append {
                            chars.next();
                        }

                        current_redirect = Some(Redirect {
                            stream: StreamType::Stderr,
                            file: String::new(),
                            append,
                        });
                        expecting_file = true;
                    } else {
                        current_arg.push(c);
                    }
                }

                '1' if !in_single_quote && !in_double_quote => {
                    if chars.peek() == Some(&'>') {
                        if !current_arg.is_empty() {
                            result.args.push(current_arg.clone());
                            current_arg.clear();
                        }

                        chars.next();
                        let append = chars.peek() == Some(&'>');
                        if append {
                            chars.next();
                        }

                        current_redirect = Some(Redirect {
                            stream: StreamType::Stdout,
                            file: String::new(),
                            append,
                        });
                        expecting_file = true;
                    } else {
                        current_arg.push(c);
                    }
                }

                '>' if !in_single_quote && !in_double_quote => {
                    if !current_arg.is_empty() {
                        result.args.push(current_arg.clone());
                        current_arg.clear();
                    }

                    let append = chars.peek() == Some(&'>');
                    if append {
                        chars.next();
                    }

                    current_redirect = Some(Redirect {
                        stream: StreamType::Stdout,
                        file: String::new(),
                        append,
                    });
                    expecting_file = true;
                }

                ' ' if !in_single_quote && !in_double_quote => {
                    if !current_arg.is_empty() {
                        result.args.push(current_arg.clone());
                        current_arg.clear();
                    }
                }

                _ => {
                    current_arg.push(c);
                }
            }
        }

        if !current_arg.is_empty() {
            if let Some(mut redirect) = current_redirect.take() {
                redirect.file = current_arg;
                result.redirects.push(redirect);
            } else {
                result.args.push(current_arg);
            }
        }

        result
    }

    fn open_redirect_file(redirect: &Redirect) -> io::Result<File> {
        if redirect.append {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&redirect.file)
        } else {
            File::create(&redirect.file)
        }
    }

    fn eval(&mut self) {
        let (command, parsed) = self.parse();

        if command.is_empty() {
            return;
        }

        // Pre-create redirect files
        for redirect in &parsed.redirects {
            let _ = Self::open_redirect_file(redirect);
        }

        match command.as_str() {
            "echo" => self.cmd_echo(&parsed),
            "type" => self.cmd_type(&parsed),
            "pwd" => self.cmd_pwd(&parsed),
            "cd" => self.cmd_cd(&parsed),
            "exit" => self.cmd_exit(&parsed),
            _ => self.cmd_external(&command, &parsed),
        }
    }

    fn write_output(&self, message: &str, parsed: &ParsedCommand) {
        for redirect in &parsed.redirects {
            if matches!(redirect.stream, StreamType::Stdout)
                && let Ok(mut file) = Self::open_redirect_file(redirect)
            {
                let _ = writeln!(file, "{}", message);
                return;
            }
        }
        println!("{}", message);
    }

    fn write_error(&self, message: &str, parsed: &ParsedCommand) {
        for redirect in &parsed.redirects {
            if matches!(redirect.stream, StreamType::Stderr)
                && let Ok(mut file) = Self::open_redirect_file(redirect)
            {
                let _ = writeln!(file, "{}", message);
                return;
            }
        }
        eprintln!("{}", message);
    }

    fn cmd_exit(&self, parsed: &ParsedCommand) -> ! {
        let code: i32 = parsed
            .args
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        std::process::exit(code);
    }

    fn cmd_echo(&self, parsed: &ParsedCommand) {
        let output = parsed.args.join(" ");
        self.write_output(&output, parsed);
    }

    fn cmd_type(&self, parsed: &ParsedCommand) {
        for cmd in &parsed.args {
            if cmd.is_empty() {
                continue;
            }

            if self.builtins.contains(cmd.as_str()) {
                self.write_output(&format!("{} is a shell builtin", cmd), parsed);
            } else if let Some(path) = self.find_executable(cmd) {
                self.write_output(&format!("{} is {}", cmd, path), parsed);
            } else {
                self.write_error(&format!("{}: not found", cmd), parsed);
            }
        }
    }

    fn cmd_pwd(&self, parsed: &ParsedCommand) {
        match env::current_dir() {
            Ok(path) => self.write_output(&path.display().to_string(), parsed),
            Err(e) => self.write_error(&format!("pwd: {}", e), parsed),
        }
    }

    fn cmd_cd(&self, parsed: &ParsedCommand) {
        let arg = parsed.args.first().map(|s| s.as_str()).unwrap_or("");

        let path = match arg {
            "" | "~" => env::var("HOME")
                .or_else(|_| env::var("USERPROFILE"))
                .unwrap_or_default(),
            path if path.starts_with("~/") => {
                let home = env::var("HOME")
                    .or_else(|_| env::var("USERPROFILE"))
                    .unwrap_or_default();
                format!("{}{}", home, &path[1..])
            }
            path => path.to_string(),
        };

        let path = Path::new(&path);

        if path.exists() {
            if let Err(e) = env::set_current_dir(path) {
                self.write_error(&format!("cd: {}: {}", path.display(), e), parsed);
            }
        } else {
            self.write_error(
                &format!("cd: {}: No such file or directory", path.display()),
                parsed,
            );
        }
    }

    fn cmd_external(&self, command: &str, parsed: &ParsedCommand) {
        if let Some(path) = self.find_executable(command) {
            let mut cmd = ProcessCommand::new(&path);
            cmd.args(&parsed.args);

            for redirect in &parsed.redirects {
                match redirect.stream {
                    StreamType::Stdout => {
                        if let Ok(file) = Self::open_redirect_file(redirect) {
                            cmd.stdout(Stdio::from(file));
                        }
                    }
                    StreamType::Stderr => {
                        if let Ok(file) = Self::open_redirect_file(redirect) {
                            cmd.stderr(Stdio::from(file));
                        }
                    }
                }
            }

            match cmd.status() {
                Ok(_) => {}
                Err(e) => self.write_error(&format!("{}: {}", command, e), parsed),
            }
        } else {
            self.write_error(&format!("{}: command not found", command), parsed);
        }
    }

    fn run(&mut self) -> io::Result<()> {
        loop {
            if !self.read_line()? {
                break;
            }

            self.eval();
        }

        Ok(())
    }
}

fn main() {
    let mut shell = Shell::new();
    if let Err(e) = shell.run() {
        eprintln!("Shell error: {}", e);
        std::process::exit(1);
    }
}
