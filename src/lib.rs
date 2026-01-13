//! # Simple Shell
//!
//! A cross-platform shell implementation in Rust that provides basic shell functionality
//! including command execution, built-in commands, tab completion, and I/O redirection.
//!
//! ## Features
//!
//! - **Built-in commands**: `echo`, `pwd`, `cd`, `type`, `exit`
//! - **External command execution**: Run any executable in PATH
//! - **Tab completion**: Auto-complete commands and show suggestions
//! - **I/O redirection**: Support for `>`, `>>`, `1>`, `2>` redirection
//! - **Cross-platform**: Works on Windows, macOS, and Linux
//! - **Line editing**: Cursor movement, backspace, delete with arrow keys
//!
//! ## Example
//!
//! ```rust
//! use shell::Shell;
//!
//! fn main() -> std::io::Result<()> {
//!     let mut shell = Shell::new();
//!     shell.run()
//! }
//! ```
//!
//! ## Architecture
//!
//! The shell is organized into several modules:
//! - [`handle`]: Input handling and line editing
//! - [`terminal`]: Terminal raw mode control
//! - [`Shell`]: Main shell implementation with command processing

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

/// Represents the type of output stream for redirection
#[derive(Debug, Clone)]
enum StreamType {
    /// Standard output (stdout)
    Stdout,
    /// Standard error (stderr)
    Stderr,
}

/// Represents a file redirection operation
#[derive(Debug, Clone)]
struct Redirect {
    /// The stream to redirect (stdout or stderr)
    stream: StreamType,
    /// The target file path
    file: String,
    /// Whether to append to the file (true) or overwrite (false)
    append: bool,
}

/// Represents a parsed command with arguments and redirections
#[derive(Debug)]
struct ParsedCommand {
    /// Command arguments (first is the command name)
    args: Vec<String>,
    /// List of file redirections to apply
    redirects: Vec<Redirect>,
}

impl ParsedCommand {
    /// Creates a new empty parsed command
    fn new() -> Self {
        Self {
            args: Vec::new(),
            redirects: Vec::new(),
        }
    }
}

/// Represents a parsed command with arguments and redirections
#[derive(Debug)]
struct Pipe {
    commands: Vec<ParsedCo>,
    /// List of file redirections to apply
    pipes: Vec<Redirect>,
}

/// The main shell implementation
///
/// Provides a REPL (Read-Eval-Print Loop) interface with support for:
/// - Built-in commands (echo, pwd, cd, type, exit)
/// - External command execution
/// - Tab completion
/// - I/O redirection
/// - Cross-platform terminal handling
///
/// # Example
///
/// ```rust
/// use shell::Shell;
///
/// let mut shell = Shell::new();
/// // This starts the interactive shell loop
/// shell.run().expect("Shell failed to run");
/// ```
pub struct Shell {
    /// Executable search paths from the PATH environment variable
    paths: Vec<String>,
    /// Set of built-in command names
    builtins: HashSet<&'static str>,
    /// Line editor for input handling and editing
    editor: LineEditor,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    /// Creates a new shell instance
    ///
    /// Initializes the shell with:
    /// - PATH environment variable parsing
    /// - Built-in command registration
    /// - Line editor setup
    ///
    /// # Example
    ///
    /// ```rust
    /// use shell::Shell;
    ///
    /// let shell = Shell::new();
    /// ```
    pub fn new() -> Self {
        Shell {
            paths: Self::parse_path(),
            builtins: HashSet::from(["echo", "exit", "type", "pwd", "cd"]),
            editor: LineEditor::new(),
        }
    }

    /// Parses the PATH environment variable into a vector of directory paths
    ///
    /// Uses the appropriate path separator for the current platform:
    /// - Windows: semicolon (`;`)
    /// - Unix-like: colon (`:`)
    ///
    /// # Returns
    ///
    /// A vector of directory paths where executables can be found
    fn parse_path() -> Vec<String> {
        let separator = if cfg!(windows) { ';' } else { ':' };

        env::var("PATH")
            .unwrap_or_default()
            .split(separator)
            .map(String::from)
            .collect()
    }

    /// Checks if a file is executable on Unix systems
    ///
    /// Uses Unix file permissions to determine if the file has execute permissions
    /// for user, group, or other.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to check
    ///
    /// # Returns
    ///
    /// `true` if the file exists and has execute permissions, `false` otherwise
    #[cfg(unix)]
    fn is_executable(path: &Path) -> bool {
        use std::os::unix::fs::PermissionsExt;

        std::fs::metadata(path)
            .map(|m| m.is_file() && (m.permissions().mode() & 0o111 != 0))
            .unwrap_or(false)
    }

    /// Checks if a file is executable on Windows systems
    ///
    /// On Windows, executability is determined by file extension rather than permissions.
    /// Recognizes common executable extensions: .exe, .bat, .cmd, .com, .ps1
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to check
    ///
    /// # Returns
    ///
    /// `true` if the file exists and has an executable extension, `false` otherwise
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

    /// Searches for an executable in the system PATH
    ///
    /// Looks through all directories in the PATH environment variable to find
    /// an executable with the given name. On Windows, also tries common executable
    /// extensions if not provided.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command name to search for
    ///
    /// # Returns
    ///
    /// `Some(String)` with the full path to the executable if found, `None` otherwise
    ///
    /// # Example
    ///
    /// ```rust
    /// # use shell::Shell;
    /// let shell = Shell::new();
    /// if let Some(path) = shell.find_executable("ls") {
    ///     println!("Found ls at: {}", path);
    /// }
    /// ```
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

    /// Finds possible command completions for tab completion
    ///
    /// Searches through built-in commands and executables in PATH to find
    /// commands that start with the given partial string.
    ///
    /// # Arguments
    ///
    /// * `partial` - The partial command name to complete
    ///
    /// # Returns
    ///
    /// A vector of possible completions, each ending with a space
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

    /// Prints the shell prompt to stdout
    fn print_prompt(&self) {
        print!("$ ");
        let _ = io::stdout().flush();
    }

    /// Redraws the current input line with cursor positioning
    ///
    /// Clears the current line and redraws it with the current buffer content,
    /// positioning the cursor at the correct location.
    fn redraw_line(&self) {
        print!("\r\x1B[K$ {}", self.editor.buffer);

        let pos = self.editor.cursor;
        let line_len = self.editor.buffer.len();
        if pos < line_len {
            print!("\r\x1B[{}C", pos + 2);
        }

        let _ = io::stdout().flush();
    }

    /// Displays available completions to the user
    ///
    /// Shows all possible completions on a new line, then redraws the prompt
    /// and current input.
    ///
    /// # Arguments
    ///
    /// * `completions` - The list of completion strings to display
    fn show_completions(&self, completions: &[String]) {
        println!();
        println!("{}", completions.join(" "));
        self.print_prompt();
        print!("{}", self.editor.buffer);
        let _ = io::stdout().flush();
    }

    /// Handles double-tab key press for showing all completions
    fn handle_double_tab(&mut self) {
        if let Some((_, _, word)) = self.editor.get_word_at_cursor() {
            let completions = self.find_completions(word);
            self.show_completions(&completions);
        }
    }

    /// Finds the longest common prefix among a list of strings
    ///
    /// Used for tab completion to determine how much of a partial command
    /// can be auto-completed without ambiguity.
    ///
    /// # Arguments
    ///
    /// * `strings` - The list of strings to find the common prefix for
    ///
    /// # Returns
    ///
    /// The longest common prefix string
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

    /// Handles single tab key press for auto-completion
    ///
    /// Attempts to complete the current word at cursor position:
    /// - If no completions: beep
    /// - If one completion: auto-complete
    /// - If multiple completions: complete common prefix and beep
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

    /// Reads a line of input from the user with line editing support
    ///
    /// Enters raw terminal mode and processes key presses to provide:
    /// - Character input and editing
    /// - Cursor movement (arrow keys, home, end)
    /// - Backspace and delete
    /// - Tab completion
    /// - Control key handling (Ctrl+C, Ctrl+D)
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if a line was read successfully
    /// - `Ok(false)` if EOF was encountered (Ctrl+D on empty line)
    /// - `Err(io::Error)` if an I/O error occurred
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

    /// Parses the current input buffer into command and arguments
    ///
    /// Splits the current line into a command name and a ParsedCommand
    /// containing arguments and redirections.
    ///
    /// # Returns
    ///
    /// A tuple of (command_name, parsed_command_with_args_and_redirects)
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

    /// Parses a command line string into arguments and redirections
    ///
    /// Handles:
    /// - Quote parsing (single and double quotes)
    /// - Escape sequences
    /// - I/O redirection operators (`>`, `>>`, `1>`, `2>`, etc.)
    /// - Argument splitting on whitespace
    ///
    /// # Arguments
    ///
    /// * `input` - The command line string to parse
    ///
    /// # Returns
    ///
    /// A ParsedCommand containing the parsed arguments and redirections
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
                '|' if !in_single_quote && !in_double_quote => {}
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

    /// Opens a file for redirection based on redirect configuration
    ///
    /// # Arguments
    ///
    /// * `redirect` - The redirection configuration specifying file and mode
    ///
    /// # Returns
    ///
    /// - `Ok(File)` if the file was opened successfully
    /// - `Err(io::Error)` if the file could not be opened
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

    /// Evaluates and executes the current command
    ///
    /// Parses the current input and dispatches to the appropriate handler:
    /// - Built-in commands (echo, type, pwd, cd, exit)
    /// - External commands (executables in PATH)
    ///
    /// Also handles I/O redirection setup before command execution.
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

    /// Writes output message, respecting stdout redirections
    ///
    /// # Arguments
    ///
    /// * `message` - The message to write
    /// * `parsed` - The parsed command containing redirection info
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

    /// Writes error message, respecting stderr redirections
    ///
    /// # Arguments
    ///
    /// * `message` - The error message to write
    /// * `parsed` - The parsed command containing redirection info
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

    /// Implements the `exit` built-in command
    ///
    /// Exits the shell process with the specified exit code.
    ///
    /// # Arguments
    ///
    /// * `parsed` - The parsed command containing optional exit code argument
    ///
    /// # Note
    ///
    /// This function never returns as it calls `std::process::exit()`
    fn cmd_exit(&self, parsed: &ParsedCommand) -> ! {
        let code: i32 = parsed
            .args
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        std::process::exit(code);
    }

    /// Implements the `echo` built-in command
    ///
    /// Prints all arguments separated by spaces to stdout (or redirected output).
    ///
    /// # Arguments
    ///
    /// * `parsed` - The parsed command containing arguments to echo
    fn cmd_echo(&self, parsed: &ParsedCommand) {
        let output = parsed.args.join(" ");
        self.write_output(&output, parsed);
    }

    /// Implements the `type` built-in command
    ///
    /// For each argument, reports whether it is:
    /// - A shell built-in command
    /// - An external executable (with full path)
    /// - Not found
    ///
    /// # Arguments
    ///
    /// * `parsed` - The parsed command containing command names to check
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

    /// Implements the `pwd` built-in command
    ///
    /// Prints the current working directory path.
    ///
    /// # Arguments
    ///
    /// * `parsed` - The parsed command (arguments ignored for pwd)
    fn cmd_pwd(&self, parsed: &ParsedCommand) {
        match env::current_dir() {
            Ok(path) => self.write_output(&path.display().to_string(), parsed),
            Err(e) => self.write_error(&format!("pwd: {}", e), parsed),
        }
    }

    /// Implements the `cd` built-in command
    ///
    /// Changes the current working directory. Supports:
    /// - `cd` or `cd ~` - Change to home directory
    /// - `cd ~/path` - Change to path relative to home directory  
    /// - `cd path` - Change to specified path
    ///
    /// # Arguments
    ///
    /// * `parsed` - The parsed command containing the target directory
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

    /// Executes an external command
    ///
    /// Searches for the command in PATH and executes it with the given arguments.
    /// Handles I/O redirection by setting up appropriate file handles.
    ///
    /// # Arguments
    ///
    /// * `command` - The command name to execute
    /// * `parsed` - The parsed command containing arguments and redirections
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

    /// Runs the main shell loop
    ///
    /// Starts the interactive shell REPL (Read-Eval-Print Loop):
    /// 1. Reads a line of input from the user
    /// 2. Parses and evaluates the command
    /// 3. Repeats until exit or EOF
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the shell exited normally
    /// - `Err(io::Error)` if an I/O error occurred during operation
    ///
    /// # Example
    ///
    /// ```rust
    /// use shell::Shell;
    ///
    /// let mut shell = Shell::new();
    /// shell.run().expect("Shell failed to run");
    /// ```
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
