use std::collections::HashSet;
use std::io::{self, Write};

#[derive(Debug)]
struct Shell {
    prompt: String,
    builtins: HashSet<&'static str>,
}

impl Shell {
    fn new() -> Self {
        Shell {
            prompt: String::new(),
            builtins: HashSet::from(["echo", "exit", "type"]),
        }
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
            _ => println!("{}: command not found", command),
        }
    }

    fn cmd_echo(&self, args: &str) {
        println!("{}", args);
    }

    fn cmd_type(&self, args: &str) {
        let cmd = args.trim();
        if self.builtins.contains(cmd) {
            println!("{} is a shell builtin", cmd);
        } else {
            println!("{}: not found", cmd);
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
