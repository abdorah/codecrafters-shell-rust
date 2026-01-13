use std::collections::HashSet;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};

pub mod handle;
pub mod terminal;
use handle::constants::Key;
use handle::lib::LineEditor;
use handle::read_key;

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

pub struct Shell {
    paths: Vec<String>,
    builtins: HashSet<&'static str>,
    editor: LineEditor,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    pub fn new() -> Self {
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

        for builtin in &self.builtins {
            if builtin.starts_with(partial) {
                completions.push(format!("{builtin} "));
            }
        }

        for dir in &self.paths {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
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
                            completions.push(format!("{name_without_ext} "));
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

        let pos = self.editor.cursor;
        let line_len = self.editor.buffer.len();
        if pos < line_len {
            print!("\r\x1B[{}C", pos + 2);
        }

        let _ = io::stdout().flush();
    }

    fn show_completions(&self, completions: &[String]) {
        println!();
        println!("{}", completions.join(" "));
        self.print_prompt();
        print!("{}", self.editor.buffer);
        let _ = io::stdout().flush();
    }

    fn handle_double_tab(&mut self) {
        if let Some((_, _, word)) = self.editor.get_word_at_cursor() {
            let completions = self.find_completions(word);
            self.show_completions(&completions);
        }
    }

    fn longest_common_prefix(strings: &[String]) -> String {
        if strings.is_empty() {
            return String::new();
        }

        if strings.len() == 1 {
            return strings[0].clone();
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

            if prefix_len == 0 {
                break;
            }
        }

        first.chars().take(prefix_len).collect()
    }

    fn handle_tab(&mut self) {
        if let Some((start, end, word)) = self.editor.get_word_at_cursor() {
            let completions = self.find_completions(word);

            match completions.len() {
                0 => {
                    print!("\x07");
                    let _ = io::stdout().flush();
                }
                1 => {
                    self.editor.replace_word(start, end, &completions[0]);
                    self.redraw_line();
                }
                _ => {
                    let lcp = Self::longest_common_prefix(&completions);

                    if lcp.len() > word.len() {
                        self.editor.replace_word(start, end, &lcp);
                        self.redraw_line();
                    }

                    print!("\x07");
                    let _ = io::stdout().flush();
                }
            }
        }
    }

    fn read_line(&mut self) -> io::Result<bool> {
        use terminal::RawMode;

        self.editor.clear();
        self.print_prompt();

        let _raw = RawMode::enable()?;
        let mut double_tab = false;
        loop {
            match read_key()? {
                None => continue,
                Some(Key::Enter) => {
                    println!();
                    return Ok(true);
                }
                Some(Key::Tab) => {
                    if !double_tab {
                        self.handle_tab();
                        double_tab = true;
                    } else {
                        self.handle_double_tab();
                        double_tab = false;
                    }
                }
                Some(Key::Backspace) => {
                    double_tab = false;
                    self.editor.backspace();
                    self.redraw_line();
                }
                Some(Key::Delete) => {
                    double_tab = false;
                    self.editor.delete();
                    self.redraw_line();
                }
                Some(Key::Left) => {
                    double_tab = false;
                    self.editor.move_left();
                    self.redraw_line();
                }
                Some(Key::Right) => {
                    double_tab = false;
                    self.editor.move_right();
                    self.redraw_line();
                }
                Some(Key::Home) | Some(Key::CtrlA) => {
                    double_tab = false;
                    self.editor.move_home();
                    self.redraw_line();
                }
                Some(Key::End) | Some(Key::CtrlE) => {
                    double_tab = false;
                    self.editor.move_end();
                    self.redraw_line();
                }
                Some(Key::CtrlC) => {
                    println!("^C");
                    self.editor.clear();
                    return Ok(true);
                }
                Some(Key::CtrlD) => {
                    double_tab = false;
                    if self.editor.buffer.is_empty() {
                        println!();
                        return Ok(false);
                    }
                }
                Some(Key::Char(ch)) => {
                    double_tab = false;
                    self.editor.insert(ch);
                    self.redraw_line();
                }
                Some(Key::Up) | Some(Key::Down) => {
                    double_tab = false;
                    // Could implement history here
                }
                Some(Key::Unknown) => {
                    double_tab = false;
                }
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
        if self.find_executable(command).is_some() {
            let mut cmd = ProcessCommand::new(command);
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

    pub fn run(&mut self) -> io::Result<()> {
        loop {
            if !self.read_line()? {
                break;
            }

            self.eval();
        }

        Ok(())
    }
}
