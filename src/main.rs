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
            builtins: HashSet::from(["echo", "exit", "type"]),
        }
    }

    fn parse_path() -> Vec<String> {
        std::env::var("PATH")
            .unwrap_or_default()
            .split(':')
            .map(String::from)
            .collect()
    }

    fn find_executable(&self, cmd: &str) -> Option<String> {
        for dir in &self.paths {
            let full_path = format!("{}/{}", dir, cmd);
            if Path::new(&full_path).exists() {
                return Some(full_path);
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
