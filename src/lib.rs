//! # Simple Shell
//!
//! A cross-platform shell implementation in Rust that provides basic shell functionality
//! including command execution, built-in commands, tab completion, I/O redirection, and pipes.
//!
//! ## Features
//!
//! - **Built-in commands**: `echo`, `pwd`, `cd`, `type`, `exit`
//! - **External command execution**: Run any executable in PATH
//! - **Tab completion**: Auto-complete commands and show suggestions
//! - **I/O redirection**: Support for `>`, `>>`, `1>`, `2>` redirection
//! - **Pipes**: Chain commands together with `|` operator
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

/// Represents a pipeline of commands connected by pipes
#[derive(Debug)]
struct Pipeline {
    /// List of commands in the pipeline
    commands: Vec<ParsedCommand>,
}

impl Pipeline {
    /// Creates a new empty pipeline
    fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Adds a command to the pipeline
    fn add_command(&mut self, command: ParsedCommand) {
        self.commands.push(command);
    }

    /// Returns true if this is a single command (no pipes)
    fn is_single_command(&self) -> bool {
        self.commands.len() <= 1
    }
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

/// The main shell implementation
///
/// Provides a REPL (Read-Eval-Print Loop) interface with support for:
/// - Built-in commands (echo, pwd, cd, type, exit)
/// - External command execution
/// - Tab completion
/// - I/O redirection
/// - Command pipelines with the `|` operator
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

    /// Parses the current input buffer into a pipeline of commands
    ///
    /// Splits the current line into commands separated by pipes, with each
    /// command containing arguments and redirections.
    ///
    /// # Returns
    ///
    /// A Pipeline containing one or more ParsedCommands
    fn parse(&self) -> Pipeline {
        Self::parse_pipeline(self.editor.buffer.trim())
    }

    /// Parses a command line string into a pipeline of commands
    ///
    /// Handles:
    /// - Quote parsing (single and double quotes)
    /// - Escape sequences
    /// - I/O redirection operators (`>`, `>>`, `1>`, `2>`, etc.)
    /// - Pipe operators (`|`) to separate commands
    /// - Argument splitting on whitespace
    ///
    /// # Arguments
    ///
    /// * `input` - The command line string to parse
    ///
    /// # Returns
    ///
    /// A Pipeline containing the parsed commands
    fn parse_pipeline(input: &str) -> Pipeline {
        let mut pipeline = Pipeline::new();
        let mut current_command = ParsedCommand::new();
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
                            current_command.redirects.push(redirect);
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
                            current_command.args.push(current_arg.clone());
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
                            current_command.args.push(current_arg.clone());
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
                        current_command.args.push(current_arg.clone());
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
                        current_command.args.push(current_arg.clone());
                        current_arg.clear();
                    }
                }
                '|' if !in_single_quote && !in_double_quote => {
                    if !current_arg.is_empty() {
                        if let Some(mut redirect) = current_redirect.take() {
                            redirect.file = current_arg.clone();
                            current_command.redirects.push(redirect);
                        } else {
                            current_command.args.push(current_arg.clone());
                        }
                        current_arg.clear();
                    }
                    if !current_command.args.is_empty() {
                        pipeline.add_command(current_command);
                        current_command = ParsedCommand::new();
                    }
                    expecting_file = false;
                    current_redirect = None;
                }
                _ => {
                    current_arg.push(c);
                }
            }
        }

        if !current_arg.is_empty() {
            if let Some(mut redirect) = current_redirect.take() {
                redirect.file = current_arg;
                current_command.redirects.push(redirect);
            } else {
                current_command.args.push(current_arg);
            }
        }

        if !current_command.args.is_empty() {
            pipeline.add_command(current_command);
        }

        pipeline
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

    /// Evaluates and executes the current command or pipeline
    ///
    /// Parses the current input and dispatches to the appropriate handler:
    /// - Single commands: Built-in commands or external commands
    /// - Pipelines: Chains commands together with pipes
    ///
    /// Also handles I/O redirection setup before command execution.
    fn eval(&mut self) {
        let pipeline = self.parse();

        if pipeline.commands.is_empty() {
            return;
        }

        if pipeline.is_single_command() {
            // Handle single command (existing logic)
            let parsed = &pipeline.commands[0];
            if parsed.args.is_empty() {
                return;
            }

            let command = &parsed.args[0];
            let args = ParsedCommand {
                args: parsed.args[1..].to_vec(),
                redirects: parsed.redirects.clone(),
            };

            // Set up redirections
            for redirect in &args.redirects {
                let _ = Self::open_redirect_file(redirect);
            }

            match command.as_str() {
                "echo" => self.cmd_echo(&args),
                "type" => self.cmd_type(&args),
                "pwd" => self.cmd_pwd(&args),
                "cd" => self.cmd_cd(&args),
                "exit" => self.cmd_exit(&args),
                _ => self.cmd_external(command, &args),
            }
        } else {
            // Handle pipeline
            self.execute_pipeline(&pipeline);
        }
    }

    /// Executes a pipeline of commands connected by pipes
    ///
    /// Creates a chain of processes where the stdout of each command
    /// is connected to the stdin of the next command. Handles both
    /// external commands and builtin commands in pipelines.
    ///
    /// # Arguments
    ///
    /// * `pipeline` - The pipeline containing commands to execute
    fn execute_pipeline(&mut self, pipeline: &Pipeline) {
        use std::process::{Child, Stdio};
        use std::io::Write;

        if pipeline.commands.is_empty() {
            return;
        }

        let mut children: Vec<Child> = Vec::new();
        let mut previous_stdout: Option<Stdio> = None;
        let mut builtin_output: Option<String> = None;

        for (i, parsed_cmd) in pipeline.commands.iter().enumerate() {
            if parsed_cmd.args.is_empty() {
                continue;
            }

            let command_name = &parsed_cmd.args[0];
            let args = &parsed_cmd.args[1..];
            let is_last = i == pipeline.commands.len() - 1;

            // Handle built-in commands in pipelines
            if self.builtins.contains(command_name.as_str()) {
                let builtin_args = ParsedCommand {
                    args: args.to_vec(),
                    redirects: parsed_cmd.redirects.clone(),
                };

                if is_last {
                    // Last command: execute builtin normally, but handle piped input
                    if let Some(input) = builtin_output.take() {
                        self.execute_builtin_with_input(command_name, &builtin_args, &input);
                    } else if let Some(stdin) = previous_stdout.take() {
                        // Read from previous external command
                        let input = self.read_from_stdio(stdin);
                        self.execute_builtin_with_input(command_name, &builtin_args, &input);
                    } else {
                        // No piped input, execute normally
                        self.execute_builtin(command_name, &builtin_args);
                    }
                    return;
                } else {
                    // Builtin in middle of pipeline: capture its output
                    let output = if let Some(input) = builtin_output.take() {
                        self.execute_builtin_capture_output(command_name, &builtin_args, Some(&input))
                    } else if let Some(stdin) = previous_stdout.take() {
                        let input = self.read_from_stdio(stdin);
                        self.execute_builtin_capture_output(command_name, &builtin_args, Some(&input))
                    } else {
                        self.execute_builtin_capture_output(command_name, &builtin_args, None)
                    };
                    
                    builtin_output = Some(output);
                    previous_stdout = None;
                    continue;
                }
            }

            // Check if command exists
            if self.find_executable(command_name).is_none() {
                eprintln!("{}: command not found", command_name);
                return;
            }

            // Create the process
            let mut cmd = ProcessCommand::new(command_name);
            cmd.args(args);

            // Set up stdin (from previous command, builtin output, or inherit)
            if let Some(input) = builtin_output.take() {
                // Previous command was a builtin, pipe its output to this command
                match cmd
                    .stdin(Stdio::piped())
                    .stdout(if is_last { Stdio::inherit() } else { Stdio::piped() })
                    .spawn() {
                    Ok(mut child) => {
                        if let Some(ref mut stdin) = child.stdin {
                            let _ = stdin.write_all(input.as_bytes());
                        }
                        
                        if !is_last {
                            previous_stdout = child.stdout.take().map(Stdio::from);
                        }
                        children.push(child);
                    }
                    Err(e) => {
                        eprintln!("{}: {}", command_name, e);
                        return;
                    }
                }
                continue;
            } else if let Some(stdin) = previous_stdout.take() {
                cmd.stdin(stdin);
            }

            // Set up stdout (pipe to next command or inherit)
            if is_last {
                // Last command: handle redirections or use inherited stdout
                for redirect in &parsed_cmd.redirects {
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
            } else {
                // Not last command: pipe stdout to next command
                cmd.stdout(Stdio::piped());
            }

            // Handle stderr redirections (not piped)
            for redirect in &parsed_cmd.redirects {
                if matches!(redirect.stream, StreamType::Stderr) {
                    if let Ok(file) = Self::open_redirect_file(redirect) {
                        cmd.stderr(Stdio::from(file));
                    }
                }
            }

            // Spawn the process
            match cmd.spawn() {
                Ok(mut child) => {
                    // Take stdout for next command if not the last
                    if !is_last {
                        previous_stdout = child.stdout.take().map(Stdio::from);
                    }
                    children.push(child);
                }
                Err(e) => {
                    eprintln!("{}: {}", command_name, e);
                    return;
                }
            }
        }

        // Wait for all processes to complete
        for mut child in children {
            let _ = child.wait();
        }
    }

    /// Reads all output from a Stdio handle
    fn read_from_stdio(&self, _stdio: Stdio) -> String {
        // This is a placeholder - converting Stdio back to readable is complex
        // In a real implementation, we'd need to use temporary files or other mechanisms
        String::new()
    }

    /// Executes a builtin command and captures its output as a string
    fn execute_builtin_capture_output(&self, command: &str, args: &ParsedCommand, _input: Option<&str>) -> String {
        use std::io::Write;
        
        // Capture output by redirecting to a string buffer
        let mut output = Vec::new();
        
        match command {
            "echo" => {
                let result = args.args.join(" ");
                output.extend_from_slice(result.as_bytes());
                output.push(b'\n');
            }
            "pwd" => {
                match std::env::current_dir() {
                    Ok(path) => {
                        output.extend_from_slice(path.display().to_string().as_bytes());
                        output.push(b'\n');
                    }
                    Err(_) => {
                        // Error handling - for pipes, we might want to pass errors through
                    }
                }
            }
            "type" => {
                for cmd in &args.args {
                    if cmd.is_empty() {
                        continue;
                    }
                    
                    let result = if self.builtins.contains(cmd.as_str()) {
                        format!("{} is a shell builtin\n", cmd)
                    } else if let Some(path) = self.find_executable(cmd) {
                        format!("{} is {}\n", cmd, path)
                    } else {
                        format!("{}: not found\n", cmd)
                    };
                    output.extend_from_slice(result.as_bytes());
                }
            }
            _ => {
                // For commands like cd and exit, they don't produce output in pipes
                // cd changes directory but doesn't output anything
                // exit would terminate the shell, which doesn't make sense in a pipe
            }
        }
        
        String::from_utf8_lossy(&output).to_string()
    }

    /// Executes a builtin command with piped input
    fn execute_builtin_with_input(&self, command: &str, args: &ParsedCommand, _input: &str) {
        match command {
            "echo" => self.cmd_echo(args),
            "type" => self.cmd_type(args),
            "pwd" => self.cmd_pwd(args),
            "cd" => self.cmd_cd(args),
            "exit" => self.cmd_exit(args),
            _ => {}
        }
        // Note: Most builtins don't actually use stdin, but some could be enhanced to do so
    }

    /// Executes a builtin command normally
    fn execute_builtin(&self, command: &str, args: &ParsedCommand) {
        match command {
            "echo" => self.cmd_echo(args),
            "type" => self.cmd_type(args),
            "pwd" => self.cmd_pwd(args),
            "cd" => self.cmd_cd(args),
            "exit" => self.cmd_exit(args),
            _ => {}
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
