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
        let message = self.prompt.trim();
        if message.is_empty() {
            return;
        }
        println!("{}: command not found", message);
    }

    fn run(&mut self) {
        loop {
            self.print_prompt();
            self.read();

            match self.prompt.trim() {
                "echo" => {
                    print!("$ ");
                    io::stdout().flush().unwrap();
                }
                "exit" => break,
                _ => continue,
            }

            self.eval();
        }
    }
}

fn main() {
    let mut command = Command::new();
    command.run();
}
