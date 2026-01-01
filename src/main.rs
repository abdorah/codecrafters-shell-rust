use std::collections::HashSet;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};

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

#[derive(Debug)]
struct Shell {
    prompt: String,
    paths: Vec<String>,
    builtins: HashSet<&'static str>,
}

impl Shell {
    fn new() -> Self {
        Shell {
            prompt: String::new(),
            paths: Self::parse_path(),
            builtins: HashSet::from(["echo", "exit", "type", "pwd", "cd"]),
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
                    matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com")
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

    fn print_prompt(&self) {
        print!("$ ");
        let _ = io::stdout().flush();
    }

    fn read(&mut self) -> bool {
        self.prompt.clear();
        match io::stdin().read_line(&mut self.prompt) {
            Ok(0) => false,
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn parse(&self) -> (String, ParsedCommand) {
        let parsed = Self::parse_arguments(self.prompt.trim());

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
                .append(redirect.append)
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

        match command.as_str() {
            "echo" => self.cmd_echo(&parsed),
            "type" => self.cmd_type(&parsed),
            "pwd" => self.cmd_pwd(&parsed),
            "cd" => self.cmd_cd(&parsed),
            "exit" => self.cmd_exit(&parsed),
            _ => self.cmd_external(&command, &parsed),
        }
    }

    // ===== Output Helpers =====

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

    // ===== Built-in Commands =====

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

            // Setup redirections
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

    // ===== Main Loop =====

    fn run(&mut self) {
        loop {
            self.print_prompt();

            if !self.read() {
                println!();
                break;
            }

            self.eval();
        }
    }
}

fn main() {
    let mut shell = Shell::new();
    shell.run();
}
