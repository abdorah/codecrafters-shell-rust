use std::collections::HashSet;
use std::io::{self, Write};

#[derive(Debug)]
struct Command {
    prompt: String,
}

impl Command {
    fn new() -> Self {
        Command {
            prompt: String::new(),
        }
    }
}

trait Repl {
    fn read(&mut self);
    fn eval(&self);
    fn print_prompt(&self);
    fn run(&mut self);
}

impl Repl for Command {
    fn print_prompt(&self) {
        print!("$ ");
        io::stdout().flush().unwrap();
    }

    fn read(&mut self) {
        self.prompt.clear();
        io::stdin().read_line(&mut self.prompt).unwrap();
    }

    fn eval(&self) {
        let mut builtin_commands: HashSet<String> = HashSet::new();
        builtin_commands.insert("type".to_string());
        builtin_commands.insert("echo".to_string());
        builtin_commands.insert("exit".to_string());
        let message = self.prompt.trim();
        if message.is_empty() {
        } else if let Some(striped_message) = message.strip_prefix("echo ") {
            println!("{}", striped_message);
        } else if message == "echo" {
            println!(); // echo with no args prints empty line
        } else if let Some(striped_message) = message.strip_prefix("type ") {
            let striped_message = striped_message.trim();
            if builtin_commands.contains(striped_message) {
                println!("{} is a shell builtin", striped_message);
            } else {
                println!("{}: not found", striped_message);
            }
        } else {
            println!("{}: command not found", message);
        }
    }

    fn run(&mut self) {
        loop {
            self.print_prompt();
            self.read();

            let message = self.prompt.trim();
            match message {
                "exit" => break,
                _ => self.eval(),
            }
        }
    }
}

fn main() {
    let mut command = Command::new();
    command.run();
}
