use std::collections::HashSet;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command as ProcessCommand;

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

    fn parse(&self) -> (&str, &str) {
        let message = self.prompt.trim();
        message.split_once(' ').unwrap_or((message, ""))
    }

    fn parse_arguments(args: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current_arg = String::new();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut chars = args.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '\\' if in_double_quote => {
                    if let Some(&next) = chars.peek() {
                        match next {
                            '"' | '\\' | '$' | '`' => {
                                current_arg.push(chars.next().unwrap());
                            }
                            _ => {
                                // Keep backslash for other characters
                                current_arg.push('\\');
                            }
                        }
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

                ' ' if !in_single_quote && !in_double_quote => {
                    if !current_arg.is_empty() {
                        result.push(current_arg.clone());
                        current_arg.clear();
                    }
                }

                _ => {
                    current_arg.push(c);
                }
            }
        }

        if !current_arg.is_empty() {
            result.push(current_arg);
        }

        result
    }

    fn eval(&mut self) {
        let (command, args) = self.parse();

        if command.is_empty() {
            return;
        }

        match command {
            "echo" => self.cmd_echo(args),
            "type" => self.cmd_type(args),
            "pwd" => self.cmd_pwd(),
            "cd" => self.cmd_cd(args),
            "exit" => self.cmd_exit(args),
            _ => self.cmd_external(command, args),
        }
    }

    fn cmd_exit(&self, args: &str) -> ! {
        let parsed = Self::parse_arguments(args);
        let code: i32 = parsed.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        std::process::exit(code);
    }

    fn cmd_echo(&self, args: &str) {
        let parsed = Self::parse_arguments(args);
        println!("{}", parsed.join(" "));
    }

    fn cmd_type(&self, args: &str) {
        let parsed = Self::parse_arguments(args);

        for cmd in parsed {
            if cmd.is_empty() {
                continue;
            }

            if self.builtins.contains(cmd.as_str()) {
                println!("{} is a shell builtin", cmd);
            } else if let Some(path) = self.find_executable(&cmd) {
                println!("{} is {}", cmd, path);
            } else {
                eprintln!("{}: not found", cmd);
            }
        }
    }

    fn cmd_pwd(&self) {
        match env::current_dir() {
            Ok(path) => println!("{}", path.display()),
            Err(e) => eprintln!("pwd: {}", e),
        }
    }

    fn cmd_cd(&self, args: &str) {
        let parsed = Self::parse_arguments(args);
        let arg = parsed.first().map(|s| s.as_str()).unwrap_or("");

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
                eprintln!("cd: {}: {}", path.display(), e);
            }
        } else {
            eprintln!("cd: {}: No such file or directory", path.display());
        }
    }

    fn cmd_external(&self, command: &str, args: &str) {
        let mut command = command.strip_suffix("'").unwrap_or(command);
        command = command.strip_prefix("'").unwrap_or(command);
        command = command.strip_suffix("\"").unwrap_or(command);
        command = command.strip_prefix("\"").unwrap_or(command);

        if self.find_executable(command).is_some() {
            let parsed = Self::parse_arguments(args);

            match ProcessCommand::new(command).args(&parsed).status() {
                Ok(_) => {}
                Err(e) => eprintln!("{}: {}", command, e),
            }
        } else {
            eprintln!("{}: command not found", command);
        }
    }

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
