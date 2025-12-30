use std::collections::HashSet;
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

        std::env::var("PATH")
            .unwrap_or_default()
            .split(separator)
            .map(String::from)
            .collect()
    }

    #[cfg(unix)]
    fn is_executable(path: &Path) -> bool {
        use std::os::unix::fs::PermissionsExt;

        match std::fs::metadata(path) {
            Ok(metadata) => metadata.is_file() && (metadata.permissions().mode() & 0o111 != 0),
            Err(_) => false,
        }
    }

    #[cfg(windows)]
    fn is_executable(path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        match path.extension() {
            Some(ext) => {
                let ext = ext.to_string_lossy().to_lowercase();
                matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com")
            }
            None => false,
        }
    }

    fn find_executable(&self, cmd: &str) -> Option<String> {
        #[cfg(windows)]
        let candidates: Vec<String> = vec![
            cmd.to_string(),
            format!("{}.exe", cmd),
            format!("{}.bat", cmd),
            format!("{}.cmd", cmd),
        ];

        #[cfg(unix)]
        let candidates: Vec<String> = vec![cmd.to_string()];

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
        io::stdout().flush().unwrap();
    }

    fn read(&mut self) {
        self.prompt.clear();
        io::stdin().read_line(&mut self.prompt).unwrap();
    }

    fn parse(&self) -> (&str, &str) {
        let message = self.prompt.trim();
        match message.split_once(' ') {
            Some((cmd, args)) => (cmd, args),
            None => (message, ""),
        }
    }

    fn eval(&self) {
        let (command, args) = self.parse();

        if command.is_empty() {
            return;
        }

        match command {
            "echo" => self.cmd_echo(args),
            "type" => self.cmd_type(args),
            "pwd" => self.cmd_pwd(),
            "cd" => self.cmd_cd(args),
            "exit" => {}
            _ => self.cmd_external(command, args),
        }
    }

    fn cmd_echo(&self, args: &str) {
        println!("{}", args);
    }

    fn cmd_type(&self, args: &str) {
        let cmd = args.trim();

        if self.builtins.contains(cmd) {
            println!("{} is a shell builtin", cmd);
        } else if let Some(path) = self.find_executable(cmd) {
            println!("{} is {}", cmd, path);
        } else {
            println!("{}: not found", cmd);
        }
    }

    fn cmd_pwd(&self) {
        println!("{}", std::env::current_dir().unwrap().display());
    }

    fn cmd_cd(&self, args: &str) {
        let path = Path::new(args.trim());

        if !(Path::new(path).exists() && std::env::set_current_dir(path).is_ok()) {
            println!("{args}: No such file or directory");
        }
    }

    fn cmd_external(&self, command: &str, args: &str) {
        if self.find_executable(command).is_some() {
            let args: Vec<&str> = if args.is_empty() {
                vec![]
            } else {
                args.split_whitespace().collect()
            };

            let _ = ProcessCommand::new(command).args(&args).status();
        } else {
            println!("{}: command not found", command);
        }
    }

    fn run(&mut self) {
        loop {
            self.print_prompt();
            self.read();

            let (command, _) = self.parse();
            if command == "exit" {
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
