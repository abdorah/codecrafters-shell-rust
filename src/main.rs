#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    print!("$ ");
    io::stdout().flush().unwrap();
    let mut message = String::new();
    io::stdin().read_line(message).unwrap();
    let message = message.trim();
    println!("{}: command not found", message);
}
